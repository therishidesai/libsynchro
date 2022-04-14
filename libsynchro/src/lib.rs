// libsynchro
// rcu_init - start cleanup thread, and initialize handle
// rcu_read_lock - bump refcount for the current generation, return handle for that generation
// rcu_read_unlock - decrement the refcount for given generation
// rcu_assign_pointer - create a new generation, assign new handle for that generation
// call_rcu - assign a deleter for a given generation, defer the deletion for the cleanup thread in init
// Note: call_rcu may be unnecessary, just use the Drop function, and writer only needs to do assign pointer

use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

const MAX_GENERATIONS: usize = 1024;

pub struct RCU {
	ptr: AtomicPtr,
	gen: AtomicUsize,
	rc:  [AtomicUsize; MAX_GENERATIONS],
}

