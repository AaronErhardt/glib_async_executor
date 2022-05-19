use glib_async_executor::Runtime;
use std::time::Duration;

#[test]
fn single_thread_rt_test() {
    let (tx, rx) = flume::unbounded();
    let runtime = Runtime::single_thread();

    for i in 0..100_usize {
        let tx = tx.clone();
        runtime.spawn(async move {
            println!("Start: {i}");
            async_std::task::sleep(Duration::from_millis(10)).await;
            tx.send(i).unwrap();
            println!("End:   {i}");
        });
    }

    std::thread::sleep(Duration::from_millis(11));
    let mut nums = Vec::new();
    for _ in 0..100 {
        let num = rx.recv_timeout(Duration::from_millis(1)).unwrap();
        nums.push(num);
    }

    nums.sort();
    for (idx, num) in nums.iter().enumerate() {
        assert_eq!(idx, *num);
    }
}

#[test]
fn handle_test() {
    let runtime = Runtime::single_thread();
    let mut handles = Vec::new();

    for i in 0..100_usize {
        handles.push(runtime.spawn(async move {
            async_std::task::sleep(Duration::from_millis(10)).await;
            i
        }));
    }

    let mut nums: Vec<usize> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    nums.sort();
    for (idx, num) in nums.iter().enumerate() {
        assert_eq!(idx, *num);
    }
}

#[test]
fn multi_thread_rt_test() {
    let (tx, rx) = flume::unbounded();
    let runtime = Runtime::multi_thread(8).unwrap();

    for i in 0..100_usize {
        let tx = tx.clone();
        runtime.spawn(async move {
            async_std::task::sleep(Duration::from_millis(10)).await;
            tx.send(i).unwrap();
        });
    }

    std::thread::sleep(Duration::from_millis(10));
    let mut nums = Vec::new();
    for _ in 0..100 {
        let num = rx.recv_timeout(Duration::from_millis(1)).unwrap();
        nums.push(num);
    }

    nums.sort();
    for (idx, num) in nums.iter().enumerate() {
        assert_eq!(idx, *num);
    }
}
