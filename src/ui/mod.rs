use std::{sync::Arc, time::Duration};
use termion::{
    event::Key,
    input::TermRead,
    raw::{IntoRawMode, RawTerminal},
};
use tokio::{runtime::Runtime, sync::mpsc::Sender, task::JoinHandle, time::sleep};
use tui::{
    backend::{Backend, TermionBackend},
    Frame, Terminal,
};

pub mod layout;

pub enum UiStateReaction<B>
where
    B: Backend,
{
    ChangeState(Box<dyn UiState<B>>),
    Exit,
}

pub trait UiState<B>: Send
where
    B: Backend,
{
    /// Called when a state is entered upon. If `Some(Duration)` is
    /// given, `on_tick` will be called every `Duration`. Otherwise,
    /// `on_tick` can still be called in situations where a redraw
    /// is required.
    fn require_ticking(&self) -> Option<Duration>;
    /// Called upon input.
    fn on_key(&mut self, key: Key) -> Option<UiStateReaction<B>>;
    /// Called upon a tick, which can happen at fixed intervals (as
    /// specified in `require_ticking`), or when a redraw is required
    /// for some reason.
    fn on_tick(&mut self) -> Option<UiStateReaction<B>>;
    /// Draw the current state to the provided buffer.
    fn draw(&self, f: &mut Frame<B>);
}

/// Events passed between the tokio update loops and `StateFSM`.
enum FsmEvent {
    /// A redraw/logic update request.
    Tick,
    /// A key was pressed.
    Key(Key),
}

enum FsmReaction {
    Exit,
}

struct StateFsm<B>
where
    B: Backend,
{
    current_state: Box<dyn UiState<B>>,
    event_tx: Sender<FsmEvent>,
    runtime: Arc<Runtime>,
    tick_handle: Option<JoinHandle<()>>,
}

impl<B> StateFsm<B>
where
    B: Backend,
{
    fn new(state: Box<dyn UiState<B>>, event_tx: Sender<FsmEvent>, runtime: Arc<Runtime>) -> Self {
        let mut fsm = StateFsm {
            current_state: state,
            event_tx,
            runtime,
            tick_handle: None,
        };
        fsm.update_tick();
        fsm
    }

    /// Called by the update loop upon an event on the event channel.
    fn event(&mut self, event: FsmEvent) -> Option<FsmReaction> {
        let reaction = match event {
            FsmEvent::Tick => self.current_state.on_tick(),
            FsmEvent::Key(k) => self.current_state.on_key(k),
        };
        if let Some(reaction) = reaction {
            match reaction {
                UiStateReaction::ChangeState(state) => {
                    self.current_state = state;
                    self.update_tick();
                    None
                }
                UiStateReaction::Exit => Some(FsmReaction::Exit),
            }
        } else {
            None
        }
    }

    /// Request of the update loop to draw to screen.
    fn draw(&self, f: &mut Frame<B>) {
        self.current_state.draw(f);
    }

    fn update_tick(&mut self) {
        if let Some(old_handle) = &self.tick_handle {
            old_handle.abort();
        }
        if let Some(duration) = self.current_state.require_ticking() {
            let tick_handle = self.runtime.spawn({
                let event_tx = self.event_tx.clone();
                async move {
                    loop {
                        sleep(duration).await;
                        if event_tx.send(FsmEvent::Tick).await.is_err() {
                            // Channel closed. Goodbye!
                            break;
                        }
                    }
                }
            });
            self.tick_handle = Some(tick_handle);
        }
        self.event_tx.blocking_send(FsmEvent::Tick).ok();
    }
}

type BackendInUse = TermionBackend<RawTerminal<std::io::Stdout>>;

pub fn run_ui(starting_state: Box<dyn UiState<BackendInUse>>) {
    // Initialize termion/tui terminal
    let stdout = std::io::stdout()
        .into_raw_mode()
        .expect("Could not get stdout in raw mode.");
    let backend = TermionBackend::new(stdout);
    let terminal = Terminal::new(backend).unwrap();

    // The tokio handler for our async tasks
    let tokio_runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_time()
            .build()
            .unwrap(),
    );

    // The channels for communication between the tokio "threads" and the FSM
    let (event_tx, event_rx) = tokio::sync::mpsc::channel::<FsmEvent>(10_usize);

    // The state, in general. Can be thought of as "scenes" in the TUI
    let state_fsm = {
        let event_tx = event_tx.clone();
        let tokio_runtime = tokio_runtime.clone();
        StateFsm::new(starting_state, event_tx, tokio_runtime)
    };

    // The tokio task responsible for detecting terminal resizes. This is done
    // in a bit of a funky way, where we just poll the terminal size every so
    // often, and fire a `Tick` event when we detect that it has changed since
    // the last poll. On any error, this task aborts.
    tokio_runtime.spawn({
        let event_tx = event_tx.clone();
        async move {
            let mut last_seen = match termion::terminal_size() {
                Ok(val) => val,
                Err(_) => return,
            };
            loop {
                let new_size = match termion::terminal_size() {
                    Ok(val) => val,
                    Err(_) => return,
                };
                if last_seen != new_size && event_tx.send(FsmEvent::Tick).await.is_err() {
                    // Main loop has hung up, goodbye!
                    break;
                }
                last_seen = new_size;
                sleep(Duration::from_millis(200)).await;
            }
        }
    });

    // Thread responsible for listening to key events (which is exposed)
    // in a blocking iterator, and dispatch the events to the main loop.
    //
    // From tokio internal documentation:
    //
    //      For technical reasons, `stdin` is
    //      implemented by using an ordinary blocking read on a separate thread, and
    //      it is impossible to cancel that read. This can make shutdown of the
    //      runtime hang until the user presses enter.
    //
    // This will therefore left to hang and expectedly stopped on next key
    // press (detecting that the channel has been dropped) or when the whole
    // program terminates.
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for key in stdin.keys().flatten() {
            if event_tx.blocking_send(FsmEvent::Key(key)).is_err() {
                // Main loop has hung up, goodbye!
                break;
            }
        }
    });

    // The tokio task responsible for managing state and drawing TUI.
    // This task maintains a Finite State Machine that consumes the event and
    // updates its internal state. Then the FSM's current state is called upon
    // to draw the current frame.
    tokio_runtime.block_on(async move {
        let mut event_rx = event_rx;
        let mut terminal = terminal;
        let mut state_fsm = state_fsm;

        terminal.clear().unwrap();
        while let Some(event) = event_rx.recv().await {
            if let Some(reaction) = state_fsm.event(event) {
                match reaction {
                    FsmReaction::Exit => break,
                }
            }
            let draw_result = terminal.draw(|f| {
                state_fsm.draw(f);
            });
            if let Err(e) = draw_result {
                println!("Failed to draw TUI with error {:?}", e)
            };
        }
        terminal.clear().unwrap();
    });
}
