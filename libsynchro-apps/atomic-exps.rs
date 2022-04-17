use libsynchro::RCU;
use rand::Rng;

use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::{thread, time};

fn main() {
    let ptr = Box::into_raw(Box::new(0 as isize));
    let r = RCU::new(ptr);
    let ar = Arc::new(r);

    // using the periodic cleanup
    // let cleanup = libsynchro::rcu_init_periodic(&ar, 10);

    // using the wakeup cleanup
    let (cleanup, wakeup_tx): (thread::JoinHandle<()>, Sender<i8>) =
        libsynchro::rcu_init_wakeup(&ar);

    let mut handles = vec![];
    let arw = Arc::clone(&ar);
    let wr_wakeup_tx = wakeup_tx.clone();
    let writer = thread::spawn(move || {
        for i in 0..10 {
            thread::sleep(time::Duration::from_millis(10));
            let d = Box::into_raw(Box::new(i as isize));
            let old_g = libsynchro::rcu_write_update(&arw, d);
            println!("gen {}, data {:?}", old_g + 1, d);
        }
        // if using the periodic cleanup use this
        // libsynchro::synchronize_rcu(&arw);
        // if using the wakeup based cleanup use this
        libsynchro::synchronize_rcu_wakeup(&arw, &wr_wakeup_tx);
    });

    // reader threads
    for i in 0..10 {
        let arr = Arc::clone(&ar);
        let thread_tx = wakeup_tx.clone();
        let handle = thread::spawn(move || {
            for _ in 0..10 {
                let mut rng = rand::thread_rng();
                thread::sleep(time::Duration::from_millis(rng.gen_range(1..10)));
                let gen = libsynchro::rcu_read_lock(&arr);
                let d = libsynchro::rcu_read_data(&arr, gen);
                println!("Reader {}: gen {}, data {:?}", i, gen, d);
                // if using the periodic cleanup
                // libsynchro::rcu_read_unlock_periodic(&arr, gen);
                // if using the wakeup cleanup
                libsynchro::rcu_read_unlock_wakeup(&arr, gen, &thread_tx);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    writer.join().unwrap();
    cleanup.join().unwrap();
}
