use arr_macro::arr;

use rand::Rng;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicIsize, AtomicPtr, AtomicBool, Ordering};
use std::{thread, time};

struct RCU<T> {
    gen: AtomicUsize,
    rc: [AtomicIsize; 1024],
    gen_data: [AtomicPtr<T>; 1024],
    done: AtomicBool,
}

impl<T> RCU<T> {
	pub fn new(ptr: *mut T) -> Self {
		let rc_arr: [AtomicIsize; 1024] = arr![AtomicIsize::new(0); 1024];
		let gen_data_arr: [AtomicPtr<T>; 1024] = arr![AtomicPtr::new(ptr); 1024];
		Self {
			gen: AtomicUsize::new(0),
			rc: rc_arr,
			gen_data: gen_data_arr, 
			done: AtomicBool::new(false),
		}
	}
}

fn rcu_init<T: 'static>(ar: &Arc<RCU<T>>) -> thread::JoinHandle<()> {
	let arc = Arc::clone(ar);
	thread::spawn(move || {
        while !arc.done.load(Ordering::Relaxed) {
            let gens = arc.gen.load(Ordering::Relaxed);
            for i in 0..gens {
                if arc.rc[i].compare_exchange(0, -1, Ordering::Release, Ordering::Relaxed) == Ok(0) {
                    let ptr = arc.gen_data[i].load(Ordering::SeqCst);
                    unsafe { Box::from_raw(ptr) };
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
            if arc.rc[i].compare_exchange(0, -1, Ordering::Release, Ordering::Relaxed) == Ok(0) {
                println!("Going to Free gen {}!", i);
                let ptr = arc.gen_data[i].load(Ordering::SeqCst);
                unsafe { Box::from_raw(ptr) };
            }
        }
    })
}

fn rcu_write_update<T: 'static>(arw: &Arc<RCU<T>>, newptr: *mut T) -> usize {
	let curr_gen = arw.gen.load(Ordering::Relaxed);
    arw.gen_data[curr_gen+1].store(newptr, Ordering::Relaxed);
	arw.gen.fetch_add(1, Ordering::AcqRel)
}

fn rcu_read_lock<T: 'static>(arr: &Arc<RCU<T>>) -> usize {
	let num = arr.gen.load(Ordering::Relaxed);
    arr.rc[num].fetch_add(1, Ordering::Relaxed);
	num
}

fn rcu_read_unlock<T: 'static>(arr: &Arc<RCU<T>>, gen: usize) {
	arr.rc[gen].fetch_sub(1, Ordering::AcqRel);
}

fn main() {
	let ptr = Box::into_raw(Box::new(0 as isize));
	let r = RCU::new(ptr);
    let ar = Arc::new(r);

    let cleanup = rcu_init(&ar);
    
    let mut handles = vec![];
    let arw = Arc::clone(&ar);
    let writer = thread::spawn(move || {
        for i in 0..10 {
            thread::sleep(time::Duration::from_millis(10));
            let d = Box::into_raw(Box::new(i as isize));
			let old_g = rcu_write_update(&arw, d);
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
                let num = rcu_read_lock(&arr);
                let d = arr.gen_data[num].load(Ordering::SeqCst);
                println!("Reader {}: gen {}, data {:?}", i, num, d);
				rcu_read_unlock(&arr, num);
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
