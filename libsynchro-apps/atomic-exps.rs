use arr_macro::arr;

use rand::Rng;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{thread, time};

struct RCU {
	data: AtomicUsize,
    gen: AtomicUsize,
    rc: [AtomicUsize; 1024],
}

fn main() {
    let rc_arr: [AtomicUsize; 1024] = arr![AtomicUsize::new(0); 1024];
    let r = RCU {
        data: AtomicUsize::new(0),
        gen: AtomicUsize::new(0),
        rc: rc_arr,
    };
    
    let ar = Arc::new(r);
    let mut handles = vec![];
    let arw = Arc::clone(&ar);
    let writer = thread::spawn(move || {
        for _ in 0..10 {
			thread::sleep(time::Duration::from_millis(10));
			let mut g = arw.gen.load(Ordering::Relaxed);
			let mut d = arw.data.load(Ordering::Relaxed);
			g +=1;
			d +=1;
			println!("gen {}, data {}", g, d);
			arw.data.store(d, Ordering::Relaxed);
			arw.gen.store(g, Ordering::Relaxed);
		}
    });
    
    for i in 0..10 {
        let arr = Arc::clone(&ar);
        let handle = thread::spawn(move || {
			for _ in 0..10 {
				let mut rng = rand::thread_rng();
				thread::sleep(time::Duration::from_millis(rng.gen_range(1..10)));
				let num = arr.gen.load(Ordering::Relaxed);
				let d = arr.data.load(Ordering::Relaxed);
				let mut rc = arr.rc[num].load(Ordering::Relaxed);
				rc += 1;
				arr.rc[num].store(rc, Ordering::Relaxed);
				println!("Reader {}: gen {}, data {}", i, num, d);
			}
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

	writer.join().unwrap();

    println!("Final Gen: {}", ar.gen.load(Ordering::Relaxed));
    // for i in 0..ar.gen.load(Ordering::Relaxed)+1 {
    //     println!("gen {}, rc: {}", i, ar.rc[i].load(Ordering::Relaxed));
    // }
}
