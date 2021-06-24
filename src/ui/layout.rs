use regex::Regex;
use std::{
    cell::RefCell,
    cmp::min,
    collections::{BTreeSet, HashMap},
    ops::Range,
    path::{Path, PathBuf},
};
use uuid::Uuid;

type Position = (u16, u16);

pub struct VisualBox {
    width: u16,
    height: u16,
}

impl VisualBox {
    pub fn new(width: u16, height: u16) -> Self {
        VisualBox { width, height }
    }
}

/// Attempt at something like TeX's distribution algorithm, where sized boxes are
/// distributed to minimize a badness that is proportional to the amount of
/// whitespace left.
///
/// # Arguments
///
/// `buffer`: the TUI buffer over which the elements are to be distributed.
///
/// `elements`: `VisualBox`es to be distributed over the buffer.
///
/// # Returns
///
/// A vector of relative positions (starting at `(0, 0)`) denoting where each element
/// should be placed to minimize badness, respectively to each index.
pub fn distribute(max_width: u16, elements: &[VisualBox]) -> Vec<Position> {
    let splits = {
        type Badness = u64;
        let mut break_memo: Vec<Option<usize>> = vec![None; elements.len()];
        let mut badness_memo: Vec<Option<Badness>> = vec![None; elements.len()];

        let compute_badness = |i: usize, j: usize| -> Badness {
            let total_width = (i..j).map(|k| elements[k].width).sum::<u16>();
            if total_width > max_width {
                std::u64::MAX
            } else {
                ((max_width - total_width) as u64).pow(3)
            }
        };

        let mut start_stack = vec![0_usize];
        let mut length_stack = vec![1_usize];
        let mut best_badness_stack = vec![Badness::MAX];
        let mut best_break_stack = vec![1_usize];

        while !start_stack.is_empty() {
            let start = start_stack.pop().unwrap();
            let length = length_stack.pop().unwrap();
            let newline_before = start + length;

            let suffix_badness = {
                if newline_before == elements.len() {
                    // There is no suffix, therefore it cannot have badness.
                    0
                } else {
                    match badness_memo[newline_before] {
                        Some(memoized) => memoized,
                        None => {
                            // Recurse:
                            // Save the current frame
                            start_stack.push(start);
                            length_stack.push(length);

                            // Prepare the recursion frame
                            start_stack.push(newline_before);
                            length_stack.push(1);
                            best_badness_stack.push(Badness::MAX);
                            best_break_stack.push(1);
                            continue;
                        }
                    }
                }
            };

            let base_badness = compute_badness(start, newline_before);
            let badness = base_badness.saturating_add(suffix_badness);

            let best_badness = *best_badness_stack.last().unwrap();
            if badness < best_badness {
                *best_badness_stack.last_mut().unwrap() = badness;
                *best_break_stack.last_mut().unwrap() = newline_before;
            }

            if newline_before == elements.len() {
                // Finished the range over possible lengths.
                // Memoize the result for this starting point,
                // and return to upper level.
                let best_badness = best_badness_stack.pop().unwrap();
                let best_break = best_break_stack.pop().unwrap();
                badness_memo[start] = Some(best_badness);
                break_memo[start] = Some(best_break);
                continue;
            }

            // Move to next element in range
            start_stack.push(start);
            length_stack.push(length + 1);
            continue;
        }

        // The splits can be obtained by following the `break_memo` map, starting at 0.
        let mut splits = Vec::<usize>::new();
        let mut head = 0;
        while head < elements.len() {
            let next_break = break_memo[head].unwrap();
            if head == elements.len() {
                break;
            }
            splits.push(next_break);
            head = next_break;
        }

        splits
    };

    let mut positions = Vec::<Position>::new();
    let mut y: u16 = 0;

    for i in 0..splits.len() {
        let split_start = if i == 0 { 0 } else { splits[i - 1] };
        let split_end = splits[i];
        let line_elements = &elements[split_start..split_end];

        let line_height = line_elements.iter().map(|x| x.height).max().unwrap();
        let content_width: u16 = line_elements.iter().map(|x| x.width).sum();
        let whitespace = (max_width - content_width) / (split_end - split_start) as u16;

        let mut filled = 0;
        for visual_box in line_elements {
            positions.push((filled, y));
            filled += visual_box.width + std::cmp::min(2, whitespace);
        }

        y += line_height;
    }

    positions
}

/// A single-line input field, with a caret.
///
/// This struct does not handle translating user input to actions on the input
/// field, but rather provides functions to act on the input.
#[derive(Clone)]
pub struct InputField {
    input_buffer: String,
    caret_position: usize,
    buffer_start: usize,
}

impl InputField {
    pub fn new() -> Self {
        let mut input_buffer = String::with_capacity(80);
        input_buffer.push(' ');

        InputField {
            input_buffer,
            caret_position: 0,
            buffer_start: 0,
        }
    }

    pub fn add_char(&mut self, c: char) {
        self.input_buffer.insert(self.caret_position, c);
        self.caret_position += 1;
    }

    pub fn backspace_char(&mut self) {
        if self.caret_position == 0 {
            return;
        }
        self.input_buffer.remove(self.caret_position - 1);
        self.caret_position = self.caret_position.saturating_sub(1);
    }

    pub fn delete_char(&mut self) {
        if self.caret_position == self.input_buffer.len() - 1 {
            return;
        }
        self.input_buffer.remove(self.caret_position);
    }

    pub fn caret_move_left(&mut self) {
        self.caret_position = self.caret_position.saturating_sub(1);
    }

    pub fn caret_move_right(&mut self) {
        self.caret_position = min(
            self.input_buffer.len().saturating_sub(1),
            self.caret_position + 1,
        );
    }

    /// Return the string that should be rendered when displaying this input field
    /// (in a `width`-wide viewport), and the character that should be highlighted/
    /// have a caret before it.
    pub fn render(&mut self, width: u16) -> (String, usize) {
        if self.caret_position < self.buffer_start + 1 {
            self.buffer_start = self.caret_position;
        } else if self.caret_position > self.buffer_start + width as usize - 1 {
            self.buffer_start = self.caret_position.saturating_sub(width as usize) + 1;
        }
        let buffer_start = self.buffer_start;
        let buffer_end = min(buffer_start + width as usize, self.input_buffer.len());
        let highlighted = self.caret_position - buffer_start;
        (
            self.input_buffer[buffer_start..buffer_end].to_string(),
            highlighted,
        )
    }

    /// Return the user input that this input field currently has.
    pub fn consume_input(&self) -> String {
        self.input_buffer[..self.input_buffer.len() - 1].to_string()
    }
}

/// Entry in the [`FileList`].
///
/// Contains information about the surrounding elements (directory-wise),
/// forming a sort of doubly linked list when in the context of a [`FileList`].
struct FileListItem {
    last_sibling: Option<Uuid>,
    next_sibling: Option<Uuid>,
    /// The UUID of the `FileListItem` corresponding to the parent directory
    /// file for this file.
    parent: Option<Uuid>,
    /// Whether this file is showing its contents. Has no meaning for
    /// files that are not directories.
    open: bool,
    path: PathBuf,
    included: bool,
}

/// A list display of a file tree, where directories in the tree can be expanded
/// and contracted.
pub struct FileList<'path> {
    base_path: &'path Path,
    /// Map from UUID keys to `FileListItem`.
    file_items: HashMap<Uuid, FileListItem>,
    /// Map from paths to UUID keys. (Typically used as an intermediate step to
    /// convert paths to the corresponding `FileListItem` in `file_items`).
    file_keys: HashMap<PathBuf, Uuid>,
    /// Files as they are displayed in the file list, as represented by their UUID
    /// keys.
    file_list: Vec<Uuid>,
    /// If a UUID is contained in this set, then its contents have been indexed
    /// previously, to at least one level of depth, and every direct child of this
    /// file has a key in `file_keys`.
    indexed: BTreeSet<Uuid>,
    pub highlight: usize,
}

pub struct FileListIterElement<'path> {
    pub path: &'path Path,
    pub included: bool,
}

impl<'path> FileList<'path> {
    pub fn new(base_path: &'path Path) -> Self {
        let mut file_items = HashMap::<Uuid, FileListItem>::new();
        let mut file_keys = HashMap::<PathBuf, Uuid>::new();
        let mut last = None;
        let mut file_list = vec![];
        for base_child in base_path
            .read_dir()
            .expect("Could not read base directory.")
            .flatten()
        {
            let key = Uuid::new_v4();
            let item = FileListItem {
                last_sibling: last,
                next_sibling: None,
                parent: None,
                open: false,
                path: base_child.path(),
                included: true,
            };
            file_items.insert(key, item);
            file_keys.insert(base_child.path(), key);
            if let Some(last) = last {
                file_items.get_mut(&last).unwrap().next_sibling = Some(key);
            }
            last = Some(key);
            file_list.push(key);
        }

        FileList {
            base_path,
            file_items,
            file_keys,
            file_list,
            indexed: BTreeSet::<Uuid>::new(),
            highlight: 0,
        }
    }

    pub fn go_up(&mut self) {
        self.highlight = self.highlight.saturating_sub(1);
    }

    pub fn go_down(&mut self) {
        self.highlight = min(
            self.highlight.saturating_add(1),
            self.file_list.len().saturating_sub(1),
        );
    }

    pub fn toggle_folder(&mut self) {
        if self.file_list.is_empty() {
            return;
        }
        let file_key = self.file_list[self.highlight];
        let file = self.file_items.get_mut(&file_key).unwrap();
        if !file.path.is_dir() {
            return;
        }
        file.open = !file.open;
        match file.open {
            true => self.expand_dir(self.highlight),
            false => self.contract_dir(self.highlight),
        }
    }

    pub fn exclude_file(&mut self) {
        todo!()
    }

    pub fn exclude_pattern(&mut self, pattern: Regex) {
        todo!()
    }

    pub fn iter_paths(
        &self,
        range: Range<usize>,
    ) -> impl Iterator<Item = FileListIterElement<'_>> + '_ {
        self.file_list[range]
            .iter()
            .map(move |id| self.file_items.get(id).unwrap())
            .map(move |item| {
                let path = item.path.strip_prefix(self.base_path).unwrap();
                FileListIterElement {
                    path,
                    included: item.included,
                }
            })
    }

    pub fn len(&self) -> usize {
        self.file_list.len()
    }

    /// Inserts the contents to the indicated element in the `file_list` into the `file_list`,
    /// indexing the contents if needed.
    ///
    /// This function expects the indicated element of the `file_list` to be a directory, and
    /// has undefined behaviour otherwise.
    fn expand_dir(&mut self, index_in_list: usize) {
        let expand_file_key = self.file_list[index_in_list];

        if !self.indexed.contains(&expand_file_key) {
            self.index_dir(&expand_file_key);
        }

        let expand_file = self.file_items.get(&expand_file_key).unwrap();
        for child_path in expand_file
            .path
            .read_dir()
            .expect("Could not read directory.")
            .flatten()
        {
            let child_path = child_path.path();
            let child_key = *self.file_keys.get(&child_path).unwrap();
            self.file_list.insert(index_in_list + 1, child_key);
        }
    }

    fn contract_dir(&mut self, index_in_list: usize) {
        todo!()
    }

    fn index_dir(&mut self, file_key: &Uuid) {
        let file_item = self.file_items.get(file_key).unwrap();

        let mut last = None;
        for child_dir in file_item
            .path
            .read_dir()
            .expect("Could not read directory.")
            .flatten()
        {
            let key = Uuid::new_v4();
            let item = FileListItem {
                last_sibling: last,
                next_sibling: None,
                parent: None,
                open: false,
                path: child_dir.path(),
                included: true,
            };
            self.file_items.insert(key, item);
            self.file_keys.insert(child_dir.path(), key);
            if let Some(last) = last {
                self.file_items.get_mut(&last).unwrap().next_sibling = Some(key);
            }
            last = Some(key);
        }

        self.indexed.insert(*file_key);
    }
}
