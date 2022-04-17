use arr_macro::arr;

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicPtr, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::{thread, time};

const MAX_GENERATIONS: usize = 1024;

pub struct RCU<T> {
    gen: AtomicUsize,
    rc: [AtomicIsize; MAX_GENERATIONS],
    gen_data: [AtomicPtr<T>; MAX_GENERATIONS],
    done: AtomicBool,
}

impl<T> RCU<T> {
    pub fn new(ptr: *mut T) -> Self {
        let rc_arr: [AtomicIsize; MAX_GENERATIONS] = arr![AtomicIsize::new(0); 1024];
        let gen_data_arr: [AtomicPtr<T>; MAX_GENERATIONS] = arr![AtomicPtr::new(ptr); 1024];
        Self {
            gen: AtomicUsize::new(0),
            rc: rc_arr,
            gen_data: gen_data_arr,
            done: AtomicBool::new(false),
        }
    }
}

// runs the cleanup thread when a read unlock signals the ref count is at 0
pub fn rcu_init_wakeup<T: 'static>(ar: &Arc<RCU<T>>) -> (thread::JoinHandle<()>, Sender<i8>) {
    let arc = Arc::clone(ar);
    let (wakeup_tx, wakeup_rx): (Sender<i8>, Receiver<i8>) = mpsc::channel();
    (
        thread::spawn(move || {
            while !arc.done.load(Ordering::Relaxed) {
                let gens = arc.gen.load(Ordering::Relaxed);
                for i in 0..gens {
                    if arc.rc[i].compare_exchange(0, -1, Ordering::Release, Ordering::Relaxed)
                        == Ok(0)
                    {
                        let ptr = arc.gen_data[i].load(Ordering::SeqCst);
                        unsafe { Box::from_raw(ptr) };
                        println!("Going to Free gen {}, ptr: {:?}!", i, ptr);
                    }
                }

                wakeup_rx.recv().unwrap();
            }

            println!("DONE!!!!");
            // Synchronize RCU will set the done flag to true and we cleanup the rest
            let gens = arc.gen.load(Ordering::Relaxed);
            for i in 0..gens + 1 {
                if arc.rc[i].compare_exchange(0, -1, Ordering::Release, Ordering::Relaxed) == Ok(0)
                {
                    println!("Going to Free gen {}!", i);
                    let ptr = arc.gen_data[i].load(Ordering::SeqCst);
                    unsafe { Box::from_raw(ptr) };
                }
            }
        }),
        wakeup_tx,
    )
}

// runs the cleanup thread for a given period (time in miliseconds)
pub fn rcu_init_periodic<T: 'static>(ar: &Arc<RCU<T>>, period: u64) -> thread::JoinHandle<()> {
    let arc = Arc::clone(ar);
    thread::spawn(move || {
        while !arc.done.load(Ordering::Relaxed) {
            let gens = arc.gen.load(Ordering::Relaxed);
            for i in 0..gens {
                if arc.rc[i].compare_exchange(0, -1, Ordering::Release, Ordering::Relaxed) == Ok(0)
                {
                    let ptr = arc.gen_data[i].load(Ordering::SeqCst);
                    unsafe { Box::from_raw(ptr) };
                    println!("Going to Free gen {}, ptr: {:?}!", i, ptr);
                }
            }
            thread::sleep(time::Duration::from_millis(period));
        }

        println!("DONE!!!!");
        // Synchronize RCU will set the done flag to true and we cleanup the rest
        let gens = arc.gen.load(Ordering::Relaxed);
        for i in 0..gens + 1 {
            if arc.rc[i].compare_exchange(0, -1, Ordering::Release, Ordering::Relaxed) == Ok(0) {
                println!("Going to Free gen {}!", i);
                let ptr = arc.gen_data[i].load(Ordering::SeqCst);
                unsafe { Box::from_raw(ptr) };
            }
        }
    })
}

pub fn rcu_write_update<T: 'static>(arw: &Arc<RCU<T>>, newptr: *mut T) -> usize {
    let curr_gen = arw.gen.load(Ordering::Relaxed);
    arw.gen_data[curr_gen + 1].store(newptr, Ordering::Relaxed);
    arw.gen.fetch_add(1, Ordering::AcqRel)
}

pub fn rcu_read_lock<T: 'static>(arr: &Arc<RCU<T>>) -> usize {
    let num = arr.gen.load(Ordering::Relaxed);
    arr.rc[num].fetch_add(1, Ordering::Relaxed);
    num
}

pub fn rcu_read_data<T: 'static>(arr: &Arc<RCU<T>>, gen: usize) -> *mut T {
    arr.gen_data[gen].load(Ordering::SeqCst)
}

pub fn rcu_read_unlock_periodic<T: 'static>(arr: &Arc<RCU<T>>, gen: usize) {
    arr.rc[gen].fetch_sub(1, Ordering::AcqRel);
}

pub fn rcu_read_unlock_wakeup<T: 'static>(arr: &Arc<RCU<T>>, gen: usize, wakeup_tx: &Sender<i8>) {
    arr.rc[gen].fetch_sub(1, Ordering::AcqRel);

    if arr.rc[gen].load(Ordering::Relaxed) == 0 {
        // will wakup the GC thread if rcu_init_wakup is used
        wakeup_tx.send(1).unwrap();
    }
}

pub fn synchronize_rcu<T: 'static>(ar: &Arc<RCU<T>>) {
    ar.done.store(true, Ordering::Relaxed);
}

pub fn synchronize_rcu_wakeup<T: 'static>(ar: &Arc<RCU<T>>, wakeup_tx: &Sender<i8>) {
    ar.done.store(true, Ordering::Relaxed);
    wakeup_tx.send(1).unwrap();
}
