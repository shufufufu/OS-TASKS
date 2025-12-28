use rand::{distributions::Alphanumeric, Rng};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

#[derive(Clone, Debug)]
struct OutputRequest {
    pid: u32,
    content: String,
    end_batch: bool,
}

#[derive(Clone, Debug)]
struct OutputBlock {
    batch_id: u64,
    items: Vec<OutputRequest>,
}

#[derive(Debug)]
enum SpoolMessage {
    Request(OutputRequest),
    Shutdown,
}

/// 设备线程：消费输出块，模拟打印机/CRT 输出
fn device_thread(rx_dev: Receiver<OutputBlock>) {
    while let Ok(block) = rx_dev.recv() {
        println!("\n=== 输出设备：接收批次 {}，条目 {} ===", block.batch_id, block.items.len());
        for (i, item) in block.items.iter().enumerate() {
            println!("[Batch {} / Item {}] 来自进程 {}: {}", block.batch_id, i + 1, item.pid, item.content);
        }
        println!("=== 批次 {} 输出完成 ===\n", block.batch_id);
    }
    println!("输出设备：通道关闭，设备退出。");
}

/// SPOOLING 输出进程：每次搬运一条信息到输出井；遇到结束标志时，
/// 将当前井中的内容形成一个输出块并发送到设备。
fn spooling_thread(rx_req: Receiver<SpoolMessage>, tx_dev: Sender<OutputBlock>) {
    let mut current_batch: Vec<OutputRequest> = Vec::new();
    let mut batch_counter: u64 = 0;

    while let Ok(msg) = rx_req.recv() {
        match msg {
            SpoolMessage::Shutdown => {
                // 若井中尚有未输出的数据，可选择进行一次尾块输出（可选）
                if !current_batch.is_empty() {
                    batch_counter += 1;
                    let block = OutputBlock {
                        batch_id: batch_counter,
                        items: std::mem::take(&mut current_batch),
                    };
                    let _ = tx_dev.send(block);
                }
                break;
            }
            SpoolMessage::Request(req) => {
                // 每次只搬运一条（模拟“每运行一次输出一项信息到输出井”）
                println!("SPOOLING：接收来自进程 {} 的一条信息，end_batch = {}", req.pid, req.end_batch);
                current_batch.push(req.clone());

                if req.end_batch {
                    batch_counter += 1;
                    let block = OutputBlock {
                        batch_id: batch_counter,
                        items: std::mem::take(&mut current_batch),
                    };
                    println!("SPOOLING：形成输出块（批次 {}，条目 {}），发送到设备", block.batch_id, block.items.len());
                    if tx_dev.send(block).is_err() {
                        eprintln!("SPOOLING：设备通道已关闭，无法输出。终止。");
                        break;
                    }
                }
            }
        }
    }
    // 关闭设备通道，提示设备退出
    drop(tx_dev);
    println!("SPOOLING：退出。");
}

/// 产生输出请求的进程：随机生成若干条消息，每个批次以结束标志收尾
fn producer_thread(pid: u32, tx_req: Sender<SpoolMessage>, batches: usize) {
    let mut rng = rand::thread_rng();

    for b in 0..batches {
        // 每个批次随机生成 3..6 条信息
        let items_in_batch = rng.gen_range(3..=6);
        for i in 0..items_in_batch {
            let len: usize = rng.gen_range(8..=16);
            let text: String = rng
                .sample_iter(&Alphanumeric)
                .take(len)
                .map(char::from)
                .collect();

            let end_batch = i == items_in_batch - 1; // 最后一条为结束标志
            let req = OutputRequest {
                pid,
                content: format!("P{}-B{}-{}", pid, b + 1, text),
                end_batch,
            };

            // 发送请求到 SPOOLING 进程
            if tx_req.send(SpoolMessage::Request(req)).is_err() {
                eprintln!("进程 {}：SPOOLING 通道关闭，停止发送。", pid);
                return;
            }

            // 随机调度：随机短暂休眠，模拟各进程随机输出
            let sleep_ms = rng.gen_range(40..=160);
            thread::sleep(Duration::from_millis(sleep_ms));
        }

        // 批次之间也随机暂停
        let pause_ms = rng.gen_range(100..=300);
        thread::sleep(Duration::from_millis(pause_ms));
    }

    println!("进程 {}：所有批次发送完毕。", pid);
}

fn main() {
    // 通道：用户进程 -> SPOOLING
    let (tx_req, rx_req) = mpsc::channel::<SpoolMessage>();
    // 通道：SPOOLING -> 设备
    let (tx_dev, rx_dev) = mpsc::channel::<OutputBlock>();

    // 启动设备线程
    let device_handle = thread::spawn(move || device_thread(rx_dev));

    // 启动 SPOOLING 输出进程
    let tx_dev_for_spool = tx_dev.clone();
    let spool_handle = thread::spawn(move || spooling_thread(rx_req, tx_dev_for_spool));

    // 启动两个产生输出请求的进程
    let tx_req_p1 = tx_req.clone();
    let p1 = thread::spawn(move || producer_thread(1, tx_req_p1, 3));

    let tx_req_p2 = tx_req.clone();
    let p2 = thread::spawn(move || producer_thread(2, tx_req_p2, 3));

    // 等待两个进程结束
    let _ = p1.join();
    let _ = p2.join();

    // 发送关机信号给 SPOOLING
    let _ = tx_req.send(SpoolMessage::Shutdown);
    // 主动丢弃发送端，促使设备线程在通道关闭后退出
    drop(tx_req);
    drop(tx_dev);

    let _ = spool_handle.join();
    let _ = device_handle.join();

    println!("\n实验6 SPOOLING 模拟结束。");
}
