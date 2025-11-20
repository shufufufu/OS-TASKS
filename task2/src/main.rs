use std::collections::VecDeque;
use std::fmt;

// 进程状态枚举
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessState {
    Ready,      // 就绪
    Running,    // 运行
    Waiting,    // 等待
    Suspended,  // 挂起
    Terminated, // 终止
}

// PCB (Process Control Block) 结构
#[derive(Debug, Clone)]
pub struct PCB {
    pub pid: u32,                    // 进程ID
    pub name: String,                // 进程名称
    pub state: ProcessState,         // 进程状态
    pub priority: u8,                // 优先级
    pub arrival_time: u32,           // 到达时间
    pub burst_time: u32,             // 运行时间
    pub remaining_time: u32,          // 剩余时间
    pub waiting_time: u32,           // 等待时间
    pub turnaround_time: u32,         // 周转时间
    pub memory_size: u32,            // 内存需求大小
    pub memory_start: Option<u32>,   // 内存起始地址
    pub next: Option<usize>,          // 指向下一个PCB的索引
    pub prev: Option<usize>,         // 指向前一个PCB的索引
}

impl PCB {
    pub fn new(pid: u32, name: String, priority: u8, arrival_time: u32, burst_time: u32, memory_size: u32) -> Self {
        Self {
            pid,
            name,
            state: ProcessState::Ready,
            priority,
            arrival_time,
            burst_time,
            remaining_time: burst_time,
            waiting_time: 0,
            turnaround_time: 0,
            memory_size,
            memory_start: None,
            next: None,
            prev: None,
        }
    }

    // 更新进程状态
    pub fn update_state(&mut self, new_state: ProcessState) {
        self.state = new_state;
    }

    // 执行一个时间片
    pub fn execute_time_slice(&mut self, time_slice: u32) -> bool {
        if self.remaining_time > time_slice {
            self.remaining_time -= time_slice;
            false // 进程未完成
        } else {
            self.remaining_time = 0;
            true // 进程完成
        }
    }

    // 增加等待时间
    pub fn increment_waiting_time(&mut self) {
        self.waiting_time += 1;
    }
}

impl fmt::Display for PCB {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PID: {}, Name: {}, State: {:?}, Priority: {}, Arrival: {}, Burst: {}, Remaining: {}, Memory: {}KB", 
               self.pid, self.name, self.state, self.priority, self.arrival_time, 
               self.burst_time, self.remaining_time, self.memory_size)
    }
}

// 伙伴系统内存块
#[derive(Debug, Clone)]
pub struct MemoryBlock {
    pub start: u32,
    pub size: u32,
    pub is_free: bool,
    pub pid: Option<u32>,
}

impl MemoryBlock {
    pub fn new(start: u32, size: u32) -> Self {
        Self {
            start,
            size,
            is_free: true,
            pid: None,
        }
    }
}

// 伙伴系统管理器
pub struct BuddySystem {
    pub blocks: Vec<MemoryBlock>,
    pub total_memory: u32,
    pub min_block_size: u32,
}

impl BuddySystem {
    pub fn new(total_memory: u32, min_block_size: u32) -> Self {
        // 强制使用 2 的幂次作为总内存和最小块大小，便于实现伙伴算法
        let total = total_memory.next_power_of_two();
        let min_bs = min_block_size.next_power_of_two();
        let mut blocks = Vec::new();
        blocks.push(MemoryBlock::new(0, total));

        Self {
            blocks,
            total_memory: total,
            min_block_size: min_bs,
        }
    }

    // 将 size 向上取最近的 2 的幂次（至少为 min_block_size）
    fn round_up_block_size(&self, size: u32) -> u32 {
        let s = size.max(self.min_block_size);
        if s.is_power_of_two() {
            s
        } else {
            s.next_power_of_two()
        }
    }

    // 分配内存（使用简化的幂次伙伴策略）
    pub fn allocate(&mut self, size: u32, pid: u32) -> Option<u32> {
        if size == 0 {
            return None;
        }

        let desired = self.round_up_block_size(size);
        if desired > self.total_memory {
            return None;
        }

        // 找到最小的空闲块，其大小 >= desired
        let mut candidate_idx: Option<usize> = None;
        let mut candidate_size = u32::MAX;

        for (i, b) in self.blocks.iter().enumerate() {
            if b.is_free && b.size >= desired {
                if b.size < candidate_size {
                    candidate_size = b.size;
                    candidate_idx = Some(i);
                }
            }
        }

        let mut idx = match candidate_idx {
            Some(i) => i,
            None => return None,
        };

        // 如果找到的块大于所需，则不断分裂直到等于
        while self.blocks[idx].size > desired {
            let cur_start = self.blocks[idx].start;
            let cur_size = self.blocks[idx].size;
            let half = cur_size / 2;

            // 将当前块变为左半块，插入右半块
            self.blocks[idx].size = half;
            self.blocks.insert(idx + 1, MemoryBlock::new(cur_start + half, half));
        }

        // 标记为已分配
        self.blocks[idx].is_free = false;
        self.blocks[idx].pid = Some(pid);
        Some(self.blocks[idx].start)
    }

    // 释放内存并尝试合并相邻的空闲块（按幂次伙伴合并原则的简化实现）
    pub fn deallocate(&mut self, start_addr: u32, pid: u32) -> bool {
        // 找到对应块
        if let Some(pos) = self.blocks.iter().position(|b| b.start == start_addr && b.pid == Some(pid)) {
            self.blocks[pos].is_free = true;
            self.blocks[pos].pid = None;

            // 反复尝试合并相邻的相同大小且对齐的空闲块
            self.merge_blocks();
            return true;
        }
        false
    }

    // 合并相邻且大小相等、对齐的空闲块（简化伙伴合并）
    fn merge_blocks(&mut self) {
        // 按起始地址排序以便判断相邻块
        self.blocks.sort_by_key(|b| b.start);

        let mut i = 0usize;
        while i + 1 < self.blocks.len() {
            let left = &self.blocks[i].clone();
            let right = &self.blocks[i + 1].clone();

            if left.is_free && right.is_free && left.size == right.size && left.start + left.size == right.start {
                // 检查对齐：左块的起始地址必须是合并后大小的倍数
                let merged_size = left.size * 2;
                if left.start % merged_size == 0 {
                    // 合并
                    self.blocks[i].size = merged_size;
                    self.blocks.remove(i + 1);
                    // 合并后仍可能与下一个块再合并，继续在同一索引检查
                    continue;
                }
            }
            i += 1;
        }
    }

    // 获取内存使用情况
    pub fn get_memory_usage(&self) -> (u32, u32) {
        let mut used = 0u32;
        let mut free = 0u32;

        for block in &self.blocks {
            if block.is_free {
                free += block.size;
            } else {
                used += block.size;
            }
        }
        (used, free)
    }
}

// 进程队列管理器
pub struct ProcessQueueManager {
    pub ready_queue: VecDeque<usize>,    // 就绪队列
    pub waiting_queue: VecDeque<usize>,  // 等待队列
    pub running_queue: VecDeque<usize>,  // 运行队列
    pub suspended_queue: VecDeque<usize>, // 挂起队列
    pub pcb_pool: Vec<Option<PCB>>,      // PCB池
    pub free_pcb_indices: VecDeque<usize>, // 空闲PCB索引
    pub next_pid: u32,                   // 下一个进程ID
}

impl ProcessQueueManager {
    pub fn new(pool_size: usize) -> Self {
        let mut free_indices = VecDeque::new();
        let mut pcb_pool = Vec::new();
        
        for i in 0..pool_size {
            free_indices.push_back(i);
            pcb_pool.push(None);
        }
        
        Self {
            ready_queue: VecDeque::new(),
            waiting_queue: VecDeque::new(),
            running_queue: VecDeque::new(),
            suspended_queue: VecDeque::new(),
            pcb_pool,
            free_pcb_indices: free_indices,
            next_pid: 1,
        }
    }

    // 创建新进程
    pub fn create_process(&mut self, name: String, priority: u8, arrival_time: u32, burst_time: u32, memory_size: u32) -> Option<usize> {
        if let Some(index) = self.free_pcb_indices.pop_front() {
            let pcb = PCB::new(self.next_pid, name, priority, arrival_time, burst_time, memory_size);
            self.pcb_pool[index] = Some(pcb);
            self.next_pid += 1;
            
            // 将进程加入就绪队列
            self.ready_queue.push_back(index);
            Some(index)
        } else {
            None // PCB池已满
        }
    }

    // 撤销进程
    pub fn terminate_process(&mut self, index: usize) -> bool {
        if self.pcb_pool[index].is_some() {
            // 从所有队列中移除
            Self::remove_from_queue(&mut self.ready_queue, index);
            Self::remove_from_queue(&mut self.waiting_queue, index);
            Self::remove_from_queue(&mut self.running_queue, index);
            Self::remove_from_queue(&mut self.suspended_queue, index);
            
            // 释放PCB
            self.pcb_pool[index] = None;
            self.free_pcb_indices.push_back(index);
            true
        } else {
            false
        }
    }

    // 挂起进程
    pub fn suspend_process(&mut self, index: usize) -> bool {
        let current_state = self.pcb_pool[index].as_ref().map(|pcb| pcb.state.clone());
        
        if let Some(state) = current_state {
            match state {
                ProcessState::Ready => {
                    Self::remove_from_queue(&mut self.ready_queue, index);
                    if let Some(pcb) = &mut self.pcb_pool[index] {
                        pcb.update_state(ProcessState::Suspended);
                    }
                    self.suspended_queue.push_back(index);
                    true
                },
                ProcessState::Running => {
                    Self::remove_from_queue(&mut self.running_queue, index);
                    if let Some(pcb) = &mut self.pcb_pool[index] {
                        pcb.update_state(ProcessState::Suspended);
                    }
                    self.suspended_queue.push_back(index);
                    true
                },
                ProcessState::Waiting => {
                    Self::remove_from_queue(&mut self.waiting_queue, index);
                    if let Some(pcb) = &mut self.pcb_pool[index] {
                        pcb.update_state(ProcessState::Suspended);
                    }
                    self.suspended_queue.push_back(index);
                    true
                },
                _ => false,
            }
        } else {
            false
        }
    }

    // 激活进程
    pub fn activate_process(&mut self, index: usize) -> bool {
        let is_suspended = self.pcb_pool[index].as_ref().map(|pcb| pcb.state == ProcessState::Suspended).unwrap_or(false);
        
        if is_suspended {
            Self::remove_from_queue(&mut self.suspended_queue, index);
            if let Some(pcb) = &mut self.pcb_pool[index] {
                pcb.update_state(ProcessState::Ready);
            }
            self.ready_queue.push_back(index);
            true
        } else {
            false
        }
    }

    // 时间片到
    pub fn time_slice_expired(&mut self, index: usize) -> bool {
        let is_running = self.pcb_pool[index].as_ref().map(|pcb| pcb.state == ProcessState::Running).unwrap_or(false);
        
        if is_running {
            Self::remove_from_queue(&mut self.running_queue, index);
            if let Some(pcb) = &mut self.pcb_pool[index] {
                pcb.update_state(ProcessState::Ready);
            }
            self.ready_queue.push_back(index);
            true
        } else {
            false
        }
    }

    // 从队列中移除进程
    fn remove_from_queue(queue: &mut VecDeque<usize>, index: usize) {
        if let Some(pos) = queue.iter().position(|&x| x == index) {
            queue.remove(pos);
        }
    }

    // 获取PCB引用
    pub fn get_pcb(&self, index: usize) -> Option<&PCB> {
        self.pcb_pool[index].as_ref()
    }

    // 获取PCB可变引用
    pub fn get_pcb_mut(&mut self, index: usize) -> Option<&mut PCB> {
        self.pcb_pool[index].as_mut()
    }

    // 获取队列状态
    pub fn get_queue_status(&self) -> (usize, usize, usize, usize, usize) {
        (
            self.ready_queue.len(),
            self.waiting_queue.len(),
            self.running_queue.len(),
            self.suspended_queue.len(),
            self.free_pcb_indices.len(),
        )
    }
}

// 调度算法枚举
#[derive(Debug, Clone)]
pub enum SchedulingAlgorithm {
    FCFS,  // 先来先服务
    SJF,   // 短作业优先
    RR,    // 时间片轮转
    Priority, // 优先级调度
}

// 进程调度器
pub struct ProcessScheduler {
    pub algorithm: SchedulingAlgorithm,
    pub time_slice: u32,
    pub current_time: u32,
}

impl ProcessScheduler {
    pub fn new(algorithm: SchedulingAlgorithm, time_slice: u32) -> Self {
        Self {
            algorithm,
            time_slice,
            current_time: 0,
        }
    }

    // 选择下一个要运行的进程
    pub fn select_next_process(&self, queue_manager: &ProcessQueueManager) -> Option<usize> {
        match self.algorithm {
            SchedulingAlgorithm::FCFS => {
                // 先来先服务：选择就绪队列中第一个进程
                queue_manager.ready_queue.front().copied()
            },
            SchedulingAlgorithm::SJF => {
                // 短作业优先：选择剩余时间最短的进程
                let mut shortest_index = None;
                let mut shortest_time = u32::MAX;
                
                for &index in &queue_manager.ready_queue {
                    if let Some(pcb) = queue_manager.get_pcb(index) {
                        if pcb.remaining_time < shortest_time {
                            shortest_time = pcb.remaining_time;
                            shortest_index = Some(index);
                        }
                    }
                }
                shortest_index
            },
            SchedulingAlgorithm::RR => {
                // 时间片轮转：选择就绪队列中第一个进程
                queue_manager.ready_queue.front().copied()
            },
            SchedulingAlgorithm::Priority => {
                // 优先级调度：选择优先级最高的进程
                let mut highest_priority_index = None;
                let mut highest_priority = u8::MIN;
                
                for &index in &queue_manager.ready_queue {
                    if let Some(pcb) = queue_manager.get_pcb(index) {
                        if pcb.priority > highest_priority {
                            highest_priority = pcb.priority;
                            highest_priority_index = Some(index);
                        }
                    }
                }
                highest_priority_index
            },
        }
    }

    // 执行调度
    pub fn schedule(&mut self, queue_manager: &mut ProcessQueueManager, memory_manager: &mut BuddySystem) -> Option<usize> {
        // 选择下一个进程
        if let Some(next_index) = self.select_next_process(queue_manager) {
            // 获取PCB信息
            let (memory_size, pid) = if let Some(pcb) = queue_manager.get_pcb(next_index) {
                (pcb.memory_size, pcb.pid)
            } else {
                return None;
            };
            
            // 从就绪队列移除
            ProcessQueueManager::remove_from_queue(&mut queue_manager.ready_queue, next_index);
            
            // 分配内存
            if let Some(memory_start) = memory_manager.allocate(memory_size, pid) {
                // 更新PCB内存地址和状态
                if let Some(pcb_mut) = queue_manager.get_pcb_mut(next_index) {
                    pcb_mut.memory_start = Some(memory_start);
                    pcb_mut.update_state(ProcessState::Running);
                }
                
                // 加入运行队列
                queue_manager.running_queue.push_back(next_index);
                self.current_time += 1;
                return Some(next_index);
            }
        }
        None
    }

    // 执行一个时间片
    pub fn execute_time_slice(&mut self, queue_manager: &mut ProcessQueueManager, memory_manager: &mut BuddySystem) -> Option<usize> {
        if let Some(&running_index) = queue_manager.running_queue.front() {
            // 获取PCB信息
            let (memory_start, pid) = if let Some(pcb) = queue_manager.get_pcb(running_index) {
                (pcb.memory_start, pcb.pid)
            } else {
                return None;
            };
            
            // 执行时间片
            let completed = if let Some(pcb) = queue_manager.get_pcb_mut(running_index) {
                pcb.execute_time_slice(self.time_slice)
            } else {
                return None;
            };
            
            if completed {
                // 进程完成
                queue_manager.running_queue.pop_front();
                
                // 释放内存
                if let Some(memory_start) = memory_start {
                    memory_manager.deallocate(memory_start, pid);
                }
                
                // 计算周转时间
                if let Some(pcb) = queue_manager.get_pcb_mut(running_index) {
                    pcb.turnaround_time = self.current_time - pcb.arrival_time;
                }
                
                // 终止进程
                queue_manager.terminate_process(running_index);
                return Some(running_index);
            } else {
                // 时间片用完，重新调度
                queue_manager.time_slice_expired(running_index);
                return self.schedule(queue_manager, memory_manager);
            }
        }
        None
    }
}

// 系统快照
#[derive(Debug)]
pub struct SystemSnapshot {
    pub current_time: u32,
    pub ready_queue: Vec<usize>,
    pub waiting_queue: Vec<usize>,
    pub running_queue: Vec<usize>,
    pub suspended_queue: Vec<usize>,
    pub free_pcb_count: usize,
    pub memory_used: u32,
    pub memory_free: u32,
}

impl SystemSnapshot {
    pub fn new(queue_manager: &ProcessQueueManager, memory_manager: &BuddySystem, current_time: u32) -> Self {
        let (memory_used, memory_free) = memory_manager.get_memory_usage();
        
        Self {
            current_time,
            ready_queue: queue_manager.ready_queue.iter().copied().collect(),
            waiting_queue: queue_manager.waiting_queue.iter().copied().collect(),
            running_queue: queue_manager.running_queue.iter().copied().collect(),
            suspended_queue: queue_manager.suspended_queue.iter().copied().collect(),
            free_pcb_count: queue_manager.free_pcb_indices.len(),
            memory_used,
            memory_free,
        }
    }

    pub fn print(&self, queue_manager: &ProcessQueueManager) {
        println!("\n=== 系统快照 (时间: {}) ===", self.current_time);
        println!("PCB池状态: {} 个空闲PCB", self.free_pcb_count);
        println!("内存使用: {}KB / {}KB", self.memory_used, self.memory_used + self.memory_free);
        
        println!("\n就绪队列 ({} 个进程):", self.ready_queue.len());
        for &index in &self.ready_queue {
            if let Some(pcb) = queue_manager.get_pcb(index) {
                println!("  {}", pcb);
            }
        }
        
        println!("\n运行队列 ({} 个进程):", self.running_queue.len());
        for &index in &self.running_queue {
            if let Some(pcb) = queue_manager.get_pcb(index) {
                println!("  {}", pcb);
            }
        }
        
        println!("\n等待队列 ({} 个进程):", self.waiting_queue.len());
        for &index in &self.waiting_queue {
            if let Some(pcb) = queue_manager.get_pcb(index) {
                println!("  {}", pcb);
            }
        }
        
        println!("\n挂起队列 ({} 个进程):", self.suspended_queue.len());
        for &index in &self.suspended_queue {
            if let Some(pcb) = queue_manager.get_pcb(index) {
                println!("  {}", pcb);
            }
        }
    }
}

// 交互式菜单系统
pub struct InteractiveMenu {
    pub queue_manager: ProcessQueueManager,
    pub memory_manager: BuddySystem,
    pub scheduler: ProcessScheduler,
}

impl InteractiveMenu {
    pub fn new() -> Self {
        Self {
            queue_manager: ProcessQueueManager::new(10),
            memory_manager: BuddySystem::new(1024, 64),
            scheduler: ProcessScheduler::new(SchedulingAlgorithm::FCFS, 2),
        }
    }

    pub fn run(&mut self) {
        println!("=== 进程PCB组织、管理与调度模拟实验 ===");
        println!("欢迎使用进程管理系统！");
        
        // 创建一些初始测试进程
        self.create_initial_processes();
        
        loop {
            self.show_menu();
            let choice = self.get_user_input();
            
            match choice.as_str() {
                "1" => self.create_process(),
                "2" => self.terminate_process(),
                "3" => self.suspend_process(),
                "4" => self.activate_process(),
                "5" => self.execute_time_slice(),
                "6" => self.show_snapshot(),
                "7" => self.change_scheduling_algorithm(),
                "8" => self.show_memory_status(),
                "9" => self.run_automatic_simulation(),
                "0" => {
                    println!("感谢使用！再见！");
                    break;
                },
                _ => println!("无效选择，请重新输入！"),
            }
        }
    }

    fn show_menu(&self) {
        println!("\n=== 主菜单 ===");
        println!("1. 创建新进程");
        println!("2. 撤销进程");
        println!("3. 挂起进程");
        println!("4. 激活进程");
        println!("5. 执行一个时间片");
        println!("6. 显示系统快照");
        println!("7. 更改调度算法");
        println!("8. 显示内存状态");
        println!("9. 运行自动模拟");
        println!("0. 退出");
        print!("请选择操作 (0-9): ");
    }

    fn get_user_input(&self) -> String {
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        input.trim().to_string()
    }

    fn create_initial_processes(&mut self) {
        let processes = vec![
            ("进程A", 3, 0, 5, 128),
            ("进程B", 2, 1, 3, 256),
            ("进程C", 1, 2, 4, 64),
            ("进程D", 4, 3, 2, 512),
        ];
        
        println!("\n创建初始测试进程...");
        for (name, priority, arrival_time, burst_time, memory_size) in processes {
            if let Some(index) = self.queue_manager.create_process(
                name.to_string(), priority, arrival_time, burst_time, memory_size
            ) {
                println!("创建进程: {}", self.queue_manager.get_pcb(index).unwrap());
            }
        }
    }

    fn create_process(&mut self) {
        println!("\n=== 创建新进程 ===");
        
        print!("进程名称: ");
        let name = self.get_user_input();
        
        print!("优先级 (1-10): ");
        let priority_input = self.get_user_input();
        let priority = priority_input.parse::<u8>().unwrap_or(5);
        
        print!("到达时间: ");
        let arrival_input = self.get_user_input();
        let arrival_time = arrival_input.parse::<u32>().unwrap_or(0);
        
        print!("运行时间: ");
        let burst_input = self.get_user_input();
        let burst_time = burst_input.parse::<u32>().unwrap_or(1);
        
        print!("内存需求 (KB): ");
        let memory_input = self.get_user_input();
        let memory_size = memory_input.parse::<u32>().unwrap_or(64);
        
        if let Some(index) = self.queue_manager.create_process(
            name, priority, arrival_time, burst_time, memory_size
        ) {
            println!("成功创建进程: {}", self.queue_manager.get_pcb(index).unwrap());
        } else {
            println!("创建失败：PCB池已满！");
        }
    }

    fn terminate_process(&mut self) {
        println!("\n=== 撤销进程 ===");
        self.show_all_processes();
        
        print!("请输入要撤销的进程索引: ");
        let input = self.get_user_input();
        if let Ok(index) = input.parse::<usize>() {
            if self.queue_manager.terminate_process(index) {
                println!("成功撤销进程 {}", index);
            } else {
                println!("撤销失败：进程不存在！");
            }
        } else {
            println!("无效的进程索引！");
        }
    }

    fn suspend_process(&mut self) {
        println!("\n=== 挂起进程 ===");
        self.show_all_processes();
        
        print!("请输入要挂起的进程索引: ");
        let input = self.get_user_input();
        if let Ok(index) = input.parse::<usize>() {
            if self.queue_manager.suspend_process(index) {
                println!("成功挂起进程 {}", index);
            } else {
                println!("挂起失败：进程不存在或状态不支持挂起！");
            }
        } else {
            println!("无效的进程索引！");
        }
    }

    fn activate_process(&mut self) {
        println!("\n=== 激活进程 ===");
        self.show_suspended_processes();
        
        print!("请输入要激活的进程索引: ");
        let input = self.get_user_input();
        if let Ok(index) = input.parse::<usize>() {
            if self.queue_manager.activate_process(index) {
                println!("成功激活进程 {}", index);
            } else {
                println!("激活失败：进程不存在或状态不支持激活！");
            }
        } else {
            println!("无效的进程索引！");
        }
    }

    fn execute_time_slice(&mut self) {
        println!("\n=== 执行时间片 ===");
        
        if let Some(completed_index) = self.scheduler.execute_time_slice(&mut self.queue_manager, &mut self.memory_manager) {
            println!("时间片执行完成，进程 {} 已完成", completed_index);
        } else {
            println!("时间片执行完成，当前无进程运行");
        }
        
        self.scheduler.current_time += 1;
    }

    fn show_snapshot(&self) {
        let snapshot = SystemSnapshot::new(&self.queue_manager, &self.memory_manager, self.scheduler.current_time);
        snapshot.print(&self.queue_manager);
    }

    fn change_scheduling_algorithm(&mut self) {
        println!("\n=== 更改调度算法 ===");
        println!("1. FCFS (先来先服务)");
        println!("2. SJF (短作业优先)");
        println!("3. RR (时间片轮转)");
        println!("4. Priority (优先级调度)");
        
        print!("请选择调度算法 (1-4): ");
        let choice = self.get_user_input();
        
        let algorithm = match choice.as_str() {
            "1" => SchedulingAlgorithm::FCFS,
            "2" => SchedulingAlgorithm::SJF,
            "3" => SchedulingAlgorithm::RR,
            "4" => SchedulingAlgorithm::Priority,
            _ => {
                println!("无效选择！");
                return;
            }
        };
        
        self.scheduler.algorithm = algorithm.clone();
        println!("调度算法已更改为: {:?}", algorithm);
    }

    fn show_memory_status(&self) {
        let (used, free) = self.memory_manager.get_memory_usage();
        println!("\n=== 内存状态 ===");
        println!("总内存: {} KB", used + free);
        println!("已使用: {} KB", used);
        println!("空闲: {} KB", free);
        println!("使用率: {:.1}%", (used as f32 / (used + free) as f32) * 100.0);
        
        println!("\n内存块详情:");
        for (i, block) in self.memory_manager.blocks.iter().enumerate() {
            let status = if block.is_free { "空闲" } else { "已用" };
            let pid_info = if let Some(pid) = block.pid {
                format!(" (PID: {})", pid)
            } else {
                String::new()
            };
            println!("  块 {}: 地址 {}-{}, 大小 {}KB, 状态: {}{}", 
                     i, block.start, block.start + block.size - 1, 
                     block.size, status, pid_info);
        }
    }

    fn run_automatic_simulation(&mut self) {
        println!("\n=== 自动模拟 ===");
        println!("将执行10个时间片的自动调度...");
        
        for i in 0..10 {
            println!("\n--- 时间片 {} ---", i + 1);
            
            if let Some(completed_index) = self.scheduler.execute_time_slice(&mut self.queue_manager, &mut self.memory_manager) {
                println!("进程 {} 完成", completed_index);
            } else {
                println!("无进程运行");
            }
            
            // 每3个时间片显示一次快照
            if (i + 1) % 3 == 0 {
                let snapshot = SystemSnapshot::new(&self.queue_manager, &self.memory_manager, self.scheduler.current_time);
                snapshot.print(&self.queue_manager);
            }
        }
        
        println!("\n自动模拟完成！");
    }

    fn show_all_processes(&self) {
        println!("\n所有进程:");
        for (i, pcb_opt) in self.queue_manager.pcb_pool.iter().enumerate() {
            if let Some(pcb) = pcb_opt {
                println!("  {}: {}", i, pcb);
            }
        }
    }

    fn show_suspended_processes(&self) {
        println!("\n挂起的进程:");
        for &index in &self.queue_manager.suspended_queue {
            if let Some(pcb) = self.queue_manager.get_pcb(index) {
                println!("  {}: {}", index, pcb);
            }
        }
    }
}

fn main() {
    let mut menu = InteractiveMenu::new();
    menu.run();
}