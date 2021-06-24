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
    depth: usize,
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
    exclude_patterns: BTreeSet<glob::Pattern>,
    exclude_exceptions: BTreeSet<Uuid>,
    exclude_explicit: BTreeSet<Uuid>,
    pub highlight: usize,
}

pub struct FileListIterElement<'path> {
    pub path: &'path Path,
    pub included: bool,
    pub depth: usize,
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
                depth: 0,
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
            exclude_patterns: BTreeSet::<glob::Pattern>::new(),
            exclude_exceptions: BTreeSet::<Uuid>::new(),
            exclude_explicit: BTreeSet::<Uuid>::new(),
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

    pub fn toggle_exclude_file(&mut self) {
        let file_key = self.file_list[self.highlight];

        match self.is_included(&file_key) {
            true => {
                // We wish to exclude the file.
                let was_exception = self.exclude_exceptions.remove(&file_key);
                // If this file was not excluded only because it was an exception
                // to a pattern, then we do not have to explicitly exclude the file,
                // because some pattern already does.
                if !was_exception {
                    self.exclude_explicit.insert(file_key);
                }
            }
            false => {
                // We wish to include the fle.
                let was_explicit = self.exclude_explicit.remove(&file_key);
                // If this file was not explicitly excluded, then it was excluded
                // as the result of a pattern, and should be included explicitly.
                if !was_explicit {
                    self.exclude_exceptions.insert(file_key);
                }
            }
        };
    }

    pub fn exclude_pattern(&mut self, pattern: &str) -> Result<(), Box<dyn std::error::Error>> {
        let pattern = glob::Pattern::new(pattern)?;
        // New ignore pattern was newly inserted, so any exceptions that match the rule are
        // no longer exceptions.
        // NOTE: This double iteration seems unavoidable, because `drain_filter` is not stabilized.
        let remove_from_exceptions = self
            .exclude_exceptions
            .iter()
            .copied()
            .filter(|id| self.exclusion_pattern_matches(&pattern, id))
            .collect::<Vec<Uuid>>();
        for k in remove_from_exceptions {
            self.exclude_exceptions.remove(&k);
        }
        // Any explicit exclusions that are covered by this pattern can be also be removed
        let remove_from_explicit = self
            .exclude_explicit
            .iter()
            .copied()
            .filter(|id| self.exclusion_pattern_matches(&pattern, id))
            .collect::<Vec<Uuid>>();
        for k in remove_from_explicit {
            self.exclude_explicit.remove(&k);
        }
        // Insert the new ignore pattern into the set (ignored if already there)
        self.exclude_patterns.insert(pattern);
        Ok(())
    }

    pub fn iter_paths(
        &self,
        range: Range<usize>,
    ) -> impl Iterator<Item = FileListIterElement<'_>> + '_ {
        self.file_list[range]
            .iter()
            .map(move |id| (id, self.file_items.get(id).unwrap()))
            .map(move |(id, item)| {
                let path = item.path.strip_prefix(self.base_path).unwrap();
                FileListIterElement {
                    path,
                    included: self.is_included(id),
                    depth: item.depth,
                }
            })
    }

    pub fn len(&self) -> usize {
        self.file_list.len()
    }

    fn is_included(&self, uuid: &Uuid) -> bool {
        let self_excluded = !self.exclude_exceptions.contains(uuid)
            && (self.exclude_explicit.contains(uuid)
                || self
                    .exclude_patterns
                    .iter()
                    .any(|pattern| self.exclusion_pattern_matches(pattern, uuid)));

        if self_excluded {
            return false;
        }

        // A file can be excluded because a parent is excluded.
        if let Some(parent) = self.file_items.get(uuid).unwrap().parent {
            return self.is_included(&parent);
        }

        true
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

    /// Removes all elements immediately following the indicated element in the `file_list`,
    /// until the `next_sibling` of the `FileListItem` is found, or until de end of the list,
    /// if `next_sibling` is `None`. This has the effect of collapsing the subtree corresponding
    /// to this file in the file list display.
    ///
    /// This function expects the indicated element of the `file_list` to be a directory, and
    /// has undefined behaviour otherwise.
    fn contract_dir(&mut self, index_in_list: usize) {
        if index_in_list == self.file_list.len() - 1 {
            // Empty folder
            return;
        }

        let contract_file_key = self.file_list[index_in_list];
        let to_remove = self.file_list[(index_in_list + 1)..]
            .iter()
            .take_while(|&id| {
                self.file_items
                    .get(id)
                    .unwrap()
                    .parent
                    .map_or(false, |pid| pid == contract_file_key)
            })
            .count();
        self.file_list
            .drain((index_in_list + 1)..(index_in_list + 1 + to_remove));
    }

    fn index_dir(&mut self, file_key: &Uuid) {
        let file_item = self.file_items.get(file_key).unwrap();

        let mut last = None;
        let child_depth = file_item.depth + 1;
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
                parent: Some(*file_key),
                open: false,
                path: child_dir.path(),
                depth: child_depth,
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

    fn exclusion_pattern_matches(&self, pattern: &glob::Pattern, id: &Uuid) -> bool {
        pattern.matches_path(
            &self
                .file_items
                .get(id)
                .unwrap()
                .path
                .strip_prefix(self.base_path)
                .unwrap(),
        )
    }
}
