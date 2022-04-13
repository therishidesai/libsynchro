use arr_macro::arr;

use rand::Rng;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, AtomicIsize, AtomicPtr, AtomicBool, Ordering};
use std::{thread, time};

struct RCU<T> {
    data: AtomicPtr<T>,
    gen: AtomicUsize,
    rc: [AtomicIsize; 1024],
    done: AtomicBool,
}

fn main() {
    let rc_arr: [AtomicIsize; 1024] = arr![AtomicIsize::new(0); 1024];
    let r = RCU {
        data: AtomicPtr::new(Box::into_raw(Box::new(0))),
        gen: AtomicUsize::new(0),
        rc: rc_arr,
        done: AtomicBool::new(false),
    };

    let ar = Arc::new(r);

    let arc = Arc::clone(&ar);
    let cleanup = thread::spawn(move || {
        while (!arc.done.load(Ordering::Relaxed)){
            for i in 0..arc.gen.load(Ordering::Relaxed) {
                if (arc.rc[i].load(Ordering::Relaxed) == 0 ) {
                    println!("Going to Free gen {}!", i);
                    let b = unsafe { Box::from_raw(arc.data.load(Ordering::Relaxed)) };
                    arc.rc[i].store(-1, Ordering::Relaxed);
                }
            }
            thread::sleep(time::Duration::from_millis(10));
            // Spin loop I know...
            // just an experiment
        }
    });
    
    let mut handles = vec![];
    let arw = Arc::clone(&ar);
    let writer = thread::spawn(move || {
        for _ in 0..10 {
            thread::sleep(time::Duration::from_millis(10));
            let mut g = arw.gen.load(Ordering::Relaxed);
            g +=1;
            arw.gen.store(g, Ordering::Relaxed);
            let d = Box::into_raw(Box::new(g));
            arw.data.swap(d, Ordering::Relaxed);
            println!("gen {}, data {:?}", g, d);
        }
        arw.done.store(true, Ordering::Relaxed);
    });
    
    for i in 0..10 {
        let arr = Arc::clone(&ar);
        let handle = thread::spawn(move || {
            for _ in 0..10 {
                let mut rng = rand::thread_rng();
                thread::sleep(time::Duration::from_millis(rng.gen_range(1..10)));
                let num = arr.gen.load(Ordering::Relaxed);
                let mut rc = arr.rc[num].load(Ordering::Relaxed);
                rc += 1;
                arr.rc[num].store(rc, Ordering::Relaxed);
                let d = arr.data.load(Ordering::Relaxed);
                let mut rc = arr.rc[num].load(Ordering::Relaxed);
                rc -= 1;
                arr.rc[num].store(rc, Ordering::Relaxed);
                println!("Reader {}: gen {}, data {:?}", i, num, d);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    writer.join().unwrap();
    cleanup.join().unwrap();

    println!("Final Gen: {}", ar.gen.load(Ordering::Relaxed));
    // for i in 0..ar.gen.load(Ordering::Relaxed)+1 {
    //     println!("gen {}, rc: {}", i, ar.rc[i].load(Ordering::Relaxed));
    // }
}
