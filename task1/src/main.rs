use std::cmp::Ordering;

/// 作业任务结构体
/// 表示一个需要调度的作业任务
#[derive(Clone, Debug)]
struct Task {
    task_id: usize,        // 任务标识符
    arrival_time: f64,     // 任务到达时间（分钟）
    execution_time: f64,   // 预估执行时间（分钟）
    start_time: Option<f64>,  // 实际开始时间
    finish_time: Option<f64>, // 实际完成时间
}

impl Task {
    /// 创建新的任务实例
    fn create(task_id: usize, arrival_time: f64, execution_time: f64) -> Self {
        Self { 
            task_id, 
            arrival_time, 
            execution_time, 
            start_time: None, 
            finish_time: None 
        }
    }

    /// 计算任务周转时间
    fn calculate_turnaround_time(&self) -> Option<f64> {
        match (self.finish_time, Some(self.arrival_time)) {
            (Some(finish), Some(arrival)) => Some(finish - arrival),
            _ => None,
        }
    }

    /// 计算任务带权周转时间
    fn calculate_weighted_turnaround_time(&self) -> Option<f64> {
        match (self.calculate_turnaround_time(), self.execution_time) {
            (Some(turnaround), exec_time) if exec_time > 0.0 => Some(turnaround / exec_time),
            _ => None,
        }
    }
}

/// 打印调度结果统计信息
fn display_scheduling_results(mut tasks: Vec<Task>, algorithm_name: &str) {
    tasks.sort_by(|a, b| a.task_id.cmp(&b.task_id));
    println!("\n========== {} ==========", algorithm_name);
    println!("任务ID\t到达\t执行\t开始\t结束\t周转\t带权周转");
    
    let mut total_turnaround = 0.0;
    let mut total_weighted_turnaround = 0.0;
    let mut task_count = 0.0;
    
    for task in &tasks {
        let start = task.start_time.map_or(-1.0, |v| v);
        let finish = task.finish_time.map_or(-1.0, |v| v);
        let turnaround = task.calculate_turnaround_time().unwrap_or(-1.0);
        let weighted_turnaround = task.calculate_weighted_turnaround_time().unwrap_or(-1.0);
        
        println!("{}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}", 
                task.task_id, task.arrival_time, task.execution_time, 
                start, finish, turnaround, weighted_turnaround);
        
        if turnaround >= 0.0 {
            total_turnaround += turnaround;
            total_weighted_turnaround += weighted_turnaround.max(0.0);
            task_count += 1.0;
        }
    }
    
    if task_count > 0.0 {
        println!("平均周转时间: {:.4}", total_turnaround / task_count);
        println!("平均带权周转时间: {:.4}", total_weighted_turnaround / task_count);
    }
}

/// 先来先服务调度算法实现
/// 按照任务到达时间顺序进行调度分配
fn execute_fcfs_scheduling(tasks: &[Task], processor_count: usize) -> Vec<Task> {
    let mut task_list: Vec<Task> = tasks.to_vec();
    // 按到达时间排序
    task_list.sort_by(|a, b| a.arrival_time.partial_cmp(&b.arrival_time).unwrap_or(Ordering::Equal));

    // 记录每个处理器的空闲时间
    let mut processor_available_time: Vec<f64> = vec![0.0; processor_count];

    for task in &mut task_list {
        // 找到最早空闲的处理器
        let (processor_index, &available_time) = processor_available_time.iter().enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(Ordering::Equal)).unwrap();
        
        // 任务开始时间 = max(到达时间, 处理器空闲时间)
        let start_time = task.arrival_time.max(available_time);
        let finish_time = start_time + task.execution_time;
        
        task.start_time = Some(start_time);
        task.finish_time = Some(finish_time);
        processor_available_time[processor_index] = finish_time;
    }
    task_list
}

/// 短作业优先调度算法实现
/// 在每次调度时选择执行时间最短的任务进行分配
fn execute_sjf_scheduling(tasks: &[Task], processor_count: usize) -> Vec<Task> {
    let mut all_tasks: Vec<Task> = tasks.to_vec();
    let total_task_count = all_tasks.len();
    
    // 按到达时间排序用于发现新到达的任务
    all_tasks.sort_by(|a, b| a.arrival_time.partial_cmp(&b.arrival_time).unwrap_or(Ordering::Equal));

    let mut current_time = 0.0f64;
    let mut completed_tasks: Vec<Task> = Vec::with_capacity(total_task_count);
    let mut ready_queue: Vec<Task> = Vec::new();
    let mut next_task_index = 0; // 下一个未放入ready队列的任务索引

    // 多处理器：跟踪各处理器的空闲时间
    let mut processor_available_time: Vec<f64> = vec![0.0; processor_count];

    // 事件驱动调度：当有处理器空闲且ready队列不空时分配任务
    while completed_tasks.len() < total_task_count {
        // 将已到达的任务加入ready队列
        while next_task_index < all_tasks.len() && all_tasks[next_task_index].arrival_time <= current_time {
            ready_queue.push(all_tasks[next_task_index].clone());
            next_task_index += 1;
        }

        // 查找空闲处理器
        let mut available_processors: Vec<usize> = processor_available_time.iter().enumerate()
            .filter(|(_, &time)| time <= current_time + 1e-9)
            .map(|(index, _)| index)
            .collect();

        if available_processors.is_empty() {
            // 没有空闲处理器
            if !ready_queue.is_empty() {
                // 所有处理器忙碌但ready队列有任务，推进到最近处理器空闲时间
                if let Some(&next_available) = processor_available_time.iter()
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal)) {
                    current_time = next_available;
                    continue;
                }
            } else {
                // ready队列为空：推进到下一个到达或下一个空闲
                let next_arrival = all_tasks.get(next_task_index).map(|task| task.arrival_time);
                let next_available = processor_available_time.iter().cloned()
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
                match (next_arrival, next_available) {
                    (Some(arrival), Some(available)) => current_time = arrival.min(available),
                    (Some(arrival), None) => current_time = arrival,
                    (None, Some(available)) => current_time = available,
                    (None, None) => break,
                }
                continue;
            }
        }

        // 有处理器空闲且ready队列可能为空
        if ready_queue.is_empty() {
            // 若ready队列为空，推进到下一个任务到达
            if let Some(task) = all_tasks.get(next_task_index) {
                current_time = task.arrival_time;
                continue;
            } else {
                break;
            }
        }

        // 对ready队列按执行时间升序选择最短任务分配
        ready_queue.sort_by(|a, b| a.execution_time.partial_cmp(&b.execution_time).unwrap_or(Ordering::Equal));

        for processor in available_processors {
            if ready_queue.is_empty() { break; }
            let mut task = ready_queue.remove(0);
            let start_time = current_time.max(task.arrival_time);
            let finish_time = start_time + task.execution_time;
            task.start_time = Some(start_time);
            task.finish_time = Some(finish_time);
            completed_tasks.push(task.clone());
            processor_available_time[processor] = finish_time;
        }

        // 推进时间到下一个事件以避免死循环
        let next_arrival = all_tasks.get(next_task_index).map(|task| task.arrival_time);
        let next_available = processor_available_time.iter().cloned()
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        match (next_arrival, next_available) {
            (Some(arrival), Some(available)) => current_time = arrival.min(available),
            (Some(arrival), None) => current_time = arrival,
            (None, Some(available)) => current_time = available,
            (None, None) => break,
        }
    }

    // 处理仍在ready队列中的任务
    if !ready_queue.is_empty() {
        for task in ready_queue.into_iter() {
            let (processor_index, &available_time) = processor_available_time.iter().enumerate()
                .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(Ordering::Equal)).unwrap();
            let start_time = task.arrival_time.max(available_time);
            let finish_time = start_time + task.execution_time;
            let mut task_copy = task.clone();
            task_copy.start_time = Some(start_time);
            task_copy.finish_time = Some(finish_time);
            processor_available_time[processor_index] = finish_time;
            completed_tasks.push(task_copy);
        }
    }

    // 结果按任务ID排序返回
    completed_tasks.sort_by(|a, b| a.task_id.cmp(&b.task_id));
    completed_tasks
}

/// 最高响应比优先调度算法实现
/// 在每次分配时选择响应比最高的任务
fn execute_hrrn_scheduling(tasks: &[Task], processor_count: usize) -> Vec<Task> {
    let mut all_tasks: Vec<Task> = tasks.to_vec();
    let total_task_count = all_tasks.len();
    all_tasks.sort_by(|a, b| a.arrival_time.partial_cmp(&b.arrival_time).unwrap_or(Ordering::Equal));

    let mut current_time = 0.0f64;
    let mut completed_tasks: Vec<Task> = Vec::with_capacity(total_task_count);
    let mut ready_queue: Vec<Task> = Vec::new();
    let mut next_task_index = 0;
    let mut processor_available_time: Vec<f64> = vec![0.0; processor_count];

    while completed_tasks.len() < total_task_count {
        while next_task_index < all_tasks.len() && all_tasks[next_task_index].arrival_time <= current_time {
            ready_queue.push(all_tasks[next_task_index].clone());
            next_task_index += 1;
        }

        let mut available_processors: Vec<usize> = processor_available_time.iter().enumerate()
            .filter(|(_, &time)| time <= current_time + 1e-9)
            .map(|(index, _)| index)
            .collect();

        if available_processors.is_empty() {
            if !ready_queue.is_empty() {
                if let Some(&next_available) = processor_available_time.iter()
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal)) {
                    current_time = next_available;
                    continue;
                }
            } else {
                let next_arrival = all_tasks.get(next_task_index).map(|task| task.arrival_time);
                let next_available = processor_available_time.iter().cloned()
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
                match (next_arrival, next_available) {
                    (Some(arrival), Some(available)) => current_time = arrival.min(available),
                    (Some(arrival), None) => current_time = arrival,
                    (None, Some(available)) => current_time = available,
                    (None, None) => break,
                }
                continue;
            }
        }

        if ready_queue.is_empty() {
            if let Some(task) = all_tasks.get(next_task_index) {
                current_time = task.arrival_time;
                continue;
            } else { break; }
        }

        // 计算响应比：(等待时间 + 执行时间) / 执行时间
        ready_queue.sort_by(|a, b| {
            let response_ratio_a = (current_time - a.arrival_time + a.execution_time) / a.execution_time;
            let response_ratio_b = (current_time - b.arrival_time + b.execution_time) / b.execution_time;
            response_ratio_b.partial_cmp(&response_ratio_a).unwrap_or(Ordering::Equal)
        });

        for processor in available_processors {
            if ready_queue.is_empty() { break; }
            let mut task = ready_queue.remove(0);
            let start_time = current_time.max(task.arrival_time);
            let finish_time = start_time + task.execution_time;
            task.start_time = Some(start_time);
            task.finish_time = Some(finish_time);
            completed_tasks.push(task.clone());
            processor_available_time[processor] = finish_time;
        }

        let next_arrival = all_tasks.get(next_task_index).map(|task| task.arrival_time);
        let next_available = processor_available_time.iter().cloned()
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        match (next_arrival, next_available) {
            (Some(arrival), Some(available)) => current_time = arrival.min(available),
            (Some(arrival), None) => current_time = arrival,
            (None, Some(available)) => current_time = available,
            (None, None) => break,
        }
    }

    if !ready_queue.is_empty() {
        for task in ready_queue.into_iter() {
            let (processor_index, &available_time) = processor_available_time.iter().enumerate()
                .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(Ordering::Equal)).unwrap();
            let start_time = task.arrival_time.max(available_time);
            let finish_time = start_time + task.execution_time;
            let mut task_copy = task.clone();
            task_copy.start_time = Some(start_time);
            task_copy.finish_time = Some(finish_time);
            processor_available_time[processor_index] = finish_time;
            completed_tasks.push(task_copy);
        }
    }

    completed_tasks.sort_by(|a, b| a.task_id.cmp(&b.task_id));
    completed_tasks
}

/// 生成第一组测试任务数据
/// 用于基本算法性能测试
fn generate_test_task_set_1() -> Vec<Task> {
    vec![
        Task::create(1, 0.0, 4.0),
        Task::create(2, 1.5, 7.0),
        Task::create(3, 3.0, 5.0),
        Task::create(4, 5.0, 6.0),
        Task::create(5, 7.0, 3.0),
    ]
}

/// 生成第二组测试任务数据
/// 包含更多样化的任务类型，用于算法性能对比分析
fn generate_test_task_set_2() -> Vec<Task> {
    vec![
        Task::create(1, 0.0, 9.0),
        Task::create(2, 0.5, 3.0),
        Task::create(3, 1.0, 10.0),
        Task::create(4, 2.0, 4.0),
        Task::create(5, 8.0, 1.0),
        Task::create(6, 8.5, 2.0),
    ]
}

fn main() {
    println!("操作系统作业调度算法实验");
    println!("================================");
    
    // 单处理器环境测试（processor_count = 1）
    let test_tasks = generate_test_task_set_1();
    println!("\n【单处理器环境调度结果】");
    
    let fcfs_single_result = execute_fcfs_scheduling(&test_tasks, 1);
    display_scheduling_results(fcfs_single_result, "FCFS算法 - 单处理器");

    let sjf_single_result = execute_sjf_scheduling(&test_tasks, 1);
    display_scheduling_results(sjf_single_result, "SJF算法 - 单处理器");

    let hrrn_single_result = execute_hrrn_scheduling(&test_tasks, 1);
    display_scheduling_results(hrrn_single_result, "HRRN算法 - 单处理器");

    // 双处理器环境测试（processor_count = 2）
    let test_tasks_dual = generate_test_task_set_1();
    println!("\n【双处理器环境调度结果】");
    
    let fcfs_dual_result = execute_fcfs_scheduling(&test_tasks_dual, 2);
    display_scheduling_results(fcfs_dual_result, "FCFS算法 - 双处理器");

    let sjf_dual_result = execute_sjf_scheduling(&test_tasks_dual, 2);
    display_scheduling_results(sjf_dual_result, "SJF算法 - 双处理器");

    let hrrn_dual_result = execute_hrrn_scheduling(&test_tasks_dual, 2);
    display_scheduling_results(hrrn_dual_result, "HRRN算法 - 双处理器");

    // 不同任务流性能对比分析
    let task_stream_a = generate_test_task_set_1();
    let task_stream_b = generate_test_task_set_2();
    println!("\n【算法性能对比分析】");
    println!("================================");
    
    let stream_a_fcfs = execute_fcfs_scheduling(&task_stream_a, 1);
    let stream_b_fcfs = execute_fcfs_scheduling(&task_stream_b, 1);
    
    display_scheduling_results(stream_a_fcfs, "任务流A - FCFS算法");
    display_scheduling_results(stream_b_fcfs, "任务流B - FCFS算法");
    
    println!("\n实验完成！");
}
