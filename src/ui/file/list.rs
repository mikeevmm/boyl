use parking_lot::RwLock;
use std::{
    cmp::min,
    collections::{BTreeSet, HashMap},
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;

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
/// and contracted, and files can be included or excluded.
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

        match self.is_id_included(&file_key) {
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
                // We wish to include the file.
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
                    included: self.is_id_included(id),
                    depth: item.depth,
                }
            })
    }

    pub fn len(&self) -> usize {
        self.file_list.len()
    }

    /// Whether a path is to be included, per the settings of the user.
    ///
    /// This function is recursive, in that if a file is not known to be included or
    /// not, a recursive call to the parent directory is made (with the guarantee that
    /// at least the base directories will be known to be included or not). This
    /// function provides memoization for the procedure, where answers are stored in
    /// `memo`.
    ///
    /// This function expects the provided path to be a subpath of `self.base_path`.
    /// If this is not the case, behaviour is undefined.
    pub fn is_included_memoized_async(
        &self,
        path: &Path,
        memo: Arc<RwLock<HashMap<PathBuf, bool>>>,
    ) -> bool {
        if let Some(answer) = {
            let lock = memo.read();
            let value = (*lock).get(path).copied();
            drop(lock);
            value
        } {
            return answer;
        }
        let answer = if let Some(id) = self.file_keys.get(path) {
            self.is_id_included(id)
        } else {
            // We have not seen this file. This may be because
            // it is in a subdirectory that was not enumerated.
            self.is_included_memoized_async(
                path.parent()
                    .expect("Expected the file path to have a parent."),
                memo.clone(),
            )
        };
        if path.is_dir() {
            let mut lock = memo.write();
            (*lock).insert(path.into(), answer);
            drop(lock);
        }
        answer
    }

    fn is_id_included(&self, uuid: &Uuid) -> bool {
        let exclude_exception = self.exclude_exceptions.contains(uuid);
        if exclude_exception {
            return true;
        }

        let self_excluded = self.exclude_explicit.contains(uuid)
            || self
                .exclude_patterns
                .iter()
                .any(|pattern| self.exclusion_pattern_matches(pattern, uuid));
        if self_excluded {
            return false;
        }

        // A file can be excluded because a parent is excluded.
        if let Some(parent) = self.file_items.get(uuid).unwrap().parent {
            return self.is_id_included(&parent);
        }

        // By default files are included.
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