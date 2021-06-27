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
