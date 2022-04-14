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
        while (!arc.done.load(Ordering::SeqCst)){
			let gens = arc.gen.load(Ordering::SeqCst);
            for i in 0..gens {
                if (arc.rc[i].compare_exchange(0, -1, Ordering::SeqCst, Ordering::SeqCst) == Ok(0)) {
					let ptr = arc.gen_data[i].load(Ordering::SeqCst);
					println!("Going to Free gen {}, ptr: {:?}!", i, ptr);
					//let b = unsafe { Box::from_raw(ptr) };
                }
            }
            thread::sleep(time::Duration::from_millis(10));
            // Spin loop I know...
            // just an experiment
        }

		println!("DONE!!!!");
		let gens = arc.gen.load(Ordering::SeqCst);
        for i in 0..gens {
            if (arc.rc[i].compare_exchange(0, -1, Ordering::SeqCst, Ordering::SeqCst) == Ok(0)) {
                println!("Going to Free gen {}!", i);
				let ptr = arc.gen_data[i].load(Ordering::SeqCst);
                let b = unsafe { Box::from_raw(ptr) };
            }
        }
    });
    
    let mut handles = vec![];
    let arw = Arc::clone(&ar);
    let writer = thread::spawn(move || {
        for _ in 0..10 {
            thread::sleep(time::Duration::from_millis(10));
            let g = arw.gen.load(Ordering::SeqCst);
            let d = Box::into_raw(Box::new(g as isize));
            let old_d = arw.data.swap(d, Ordering::SeqCst);
			let old_g = arw.gen.fetch_add(1, Ordering::SeqCst);
			arw.gen_data[old_g].swap(old_d, Ordering::SeqCst);
            println!("gen {}, data {:?}", g, d);
        }
        arw.done.store(true, Ordering::SeqCst);
    });
    
    for i in 0..10 {
        let arr = Arc::clone(&ar);
        let handle = thread::spawn(move || {
            for _ in 0..10 {
                let mut rng = rand::thread_rng();
                thread::sleep(time::Duration::from_millis(rng.gen_range(1..10)));
                let num = arr.gen.load(Ordering::SeqCst);
				arr.rc[num].fetch_add(1, Ordering::SeqCst);
                let d = arr.data.load(Ordering::SeqCst);
                println!("Reader {}: gen {}, data {:?}", i, num, d);
				arr.rc[num].fetch_sub(1, Ordering::SeqCst);
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
