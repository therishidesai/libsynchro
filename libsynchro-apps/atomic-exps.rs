use arr_macro::arr;

use rand::Rng;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, AtomicIsize, AtomicPtr, AtomicBool, Ordering};
use std::{thread, time};

struct RCU<T> {
    data: AtomicPtr<T>,
    gen: AtomicUsize,
    rc: [AtomicIsize; 1024],
    gen_data: [AtomicPtr<T>; 1024],
    done: AtomicBool,
}

fn main() {
    let rc_arr: [AtomicIsize; 1024] = arr![AtomicIsize::new(0); 1024];
    let ptr = &mut 0;
    let gen_data_arr: [AtomicPtr<isize>; 1024] = arr![AtomicPtr::new(ptr); 1024];
    let r = RCU {
        data: AtomicPtr::new(Box::into_raw(Box::new(0))),
        gen: AtomicUsize::new(0),
        rc: rc_arr,
        gen_data: gen_data_arr, 
        done: AtomicBool::new(false),
    };

    let ar = Arc::new(r);

    let arc = Arc::clone(&ar);
    let cleanup = thread::spawn(move || {
        while (!arc.done.load(Ordering::Relaxed)){
            let gens = arc.gen.load(Ordering::Relaxed);
            for i in 0..gens {
                if (arc.rc[i].compare_exchange(0, -1, Ordering::Release, Ordering::Relaxed) == Ok(0)) {
                    let ptr = arc.gen_data[i].load(Ordering::SeqCst);
                    let b = unsafe { Box::from_raw(ptr) };
                    println!("Going to Free gen {}, ptr: {:?}!", i, ptr);
                }
            }
            thread::sleep(time::Duration::from_millis(10));
            // Spin loop I know...
            // just an experiment
        }

        println!("DONE!!!!");
        let gens = arc.gen.load(Ordering::Relaxed);
        for i in 0..gens {
            if (arc.rc[i].compare_exchange(0, -1, Ordering::Release, Ordering::Relaxed) == Ok(0)) {
                println!("Going to Free gen {}!", i);
                let ptr = arc.gen_data[i].load(Ordering::SeqCst);
                let b = unsafe { Box::from_raw(ptr) };
            }
        }
        
        let ptr = arc.data.load(Ordering::SeqCst);
        let b = unsafe { Box::from_raw(ptr) };
    });
    
    let mut handles = vec![];
    let arw = Arc::clone(&ar);
    let writer = thread::spawn(move || {
        for _ in 0..10 {
            thread::sleep(time::Duration::from_millis(10));
            let old_g = arw.gen.fetch_add(1, Ordering::AcqRel);
            let d = Box::into_raw(Box::new(old_g as isize));
            let old_d = arw.data.swap(d, Ordering::AcqRel);
            arw.gen_data[old_g].store(old_d, Ordering::Relaxed);
            println!("gen {}, data {:?}", old_g+1, d);
        }
        arw.done.store(true, Ordering::SeqCst);
    });

	// reader threads
    for i in 0..10 {
        let arr = Arc::clone(&ar);
        let handle = thread::spawn(move || {
            for _ in 0..10 {
                let mut rng = rand::thread_rng();
                thread::sleep(time::Duration::from_millis(rng.gen_range(1..10)));
                let num = arr.gen.load(Ordering::Relaxed);
                arr.rc[num].fetch_add(1, Ordering::Relaxed);
                let d = arr.data.load(Ordering::SeqCst);
                println!("Reader {}: gen {}, data {:?}", i, num, d);
                arr.rc[num].fetch_sub(1, Ordering::AcqRel);
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
