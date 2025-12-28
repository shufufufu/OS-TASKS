use std::collections::{HashMap, HashSet, VecDeque};

fn simulate_fifo(reference: &[i32], frames: usize) -> usize {
    assert!(frames > 0, "页框数必须大于 0");
    let mut faults = 0;
    let mut frame_queue: VecDeque<i32> = VecDeque::new();
    let mut in_memory: HashSet<i32> = HashSet::new();

    for &page in reference {
        if in_memory.contains(&page) {
            continue;
        }
        faults += 1;
        if frame_queue.len() == frames {
            if let Some(evicted) = frame_queue.pop_front() {
                in_memory.remove(&evicted);
            }
        }
        frame_queue.push_back(page);
        in_memory.insert(page);
    }

    faults
}

fn simulate_lru(reference: &[i32], frames: usize) -> usize {
    assert!(frames > 0, "页框数必须大于 0");
    let mut faults = 0;
    let mut frames_vec: Vec<i32> = Vec::new();
    let mut last_used: HashMap<i32, usize> = HashMap::new();

    for (idx, &page) in reference.iter().enumerate() {
        if frames_vec.contains(&page) {
            last_used.insert(page, idx);
            continue;
        }

        faults += 1;
        if frames_vec.len() < frames {
            frames_vec.push(page);
        } else {
            let (replace_idx, _) = frames_vec
                .iter()
                .enumerate()
                .min_by_key(|(_, &p)| last_used.get(&p).copied().unwrap_or(0))
                .expect("至少有一个页框");
            frames_vec[replace_idx] = page;
        }
        last_used.insert(page, idx);
    }

    faults
}

fn simulate_opt(reference: &[i32], frames: usize) -> usize {
    assert!(frames > 0, "页框数必须大于 0");
    let mut faults = 0;
    let mut frames_vec: Vec<i32> = Vec::new();

    for (idx, &page) in reference.iter().enumerate() {
        if frames_vec.contains(&page) {
            continue;
        }

        faults += 1;
        if frames_vec.len() < frames {
            frames_vec.push(page);
            continue;
        }

        let mut target_idx = 0;
        let mut farthest_distance: Option<usize> = None;
        for (frame_idx, &frame_page) in frames_vec.iter().enumerate() {
            match reference[idx + 1..].iter().position(|&p| p == frame_page) {
                None => {
                    target_idx = frame_idx;
                    break;
                }
                Some(dist) => {
                    let dist = dist + 1;
                    if farthest_distance.map_or(true, |current| dist > current) {
                        farthest_distance = Some(dist);
                        target_idx = frame_idx;
                    }
                }
            }
        }
        frames_vec[target_idx] = page;
    }

    faults
}

fn print_table(title: &str, labels: &[&str], values: &[usize]) {
    println!("\n{}", title);
    println!("{:-<1$}", "", 40);
    for (label, value) in labels.iter().zip(values.iter()) {
        println!("{:<20} | {:>5}", label, value);
    }
}

fn main() {
    let reference = vec![1, 2, 3, 2, 4, 1, 5, 2, 1, 2, 3, 4, 5, 2, 1];
    let frames_for_compare = 3;

    let fifo_faults = simulate_fifo(&reference, frames_for_compare);
    let lru_faults = simulate_lru(&reference, frames_for_compare);
    let opt_faults = simulate_opt(&reference, frames_for_compare);

    print_table(
        "(1) 同一批数据下 (页框=3) 的缺页次数对比",
        &["FIFO", "LRU", "OPT(参考)"],
        &[fifo_faults, lru_faults, opt_faults],
    );

    let frame_sizes = [2usize, 3, 4];
    let fifo_results: Vec<usize> = frame_sizes
        .iter()
        .map(|&k| simulate_fifo(&reference, k))
        .collect();
    print_table(
        "(2) FIFO 在不同页框数下的缺页次数",
        &["页框=2", "页框=3", "页框=4"],
        &fifo_results,
    );

    let lru_results: Vec<usize> = frame_sizes
        .iter()
        .map(|&k| simulate_lru(&reference, k))
        .collect();
    print_table(
        "(3) LRU 在不同页框数下的缺页次数",
        &["页框=2", "页框=3", "页框=4"],
        &lru_results,
    );

    let belady_sequence = vec![1, 2, 3, 4, 1, 2, 5, 1, 2, 3, 4, 5];
    let belady_fifo_3 = simulate_fifo(&belady_sequence, 3);
    let belady_fifo_4 = simulate_fifo(&belady_sequence, 4);
    print_table(
        "(4) FIFO Belady 现象示例",
        &["页框=3", "页框=4"],
        &[belady_fifo_3, belady_fifo_4],
    );
}
