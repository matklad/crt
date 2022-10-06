use std::{
    io,
    num::NonZeroUsize,
    sync::{mpsc, Condvar, Mutex},
    thread::{available_parallelism, JoinHandle},
};

pub(crate) struct Threads {
    senders: Vec<mpsc::Sender<Job<'static>>>,
    handles: Vec<JoinHandle<()>>,
}

impl Threads {
    pub(crate) fn new(n_threads: NonZeroUsize) -> Threads {
        let n_threads = n_threads.get();
        let mut res = Threads {
            senders: Vec::with_capacity(n_threads),
            handles: Vec::with_capacity(n_threads),
        };
        for _ in 0..n_threads {
            let (sender, receiver) = mpsc::channel::<Job>();
            let handle = std::thread::spawn(move || {
                for job in receiver {
                    (job.f)()
                }
            });
            res.senders.push(sender);
            res.handles.push(handle)
        }
        res
    }
    pub(crate) fn with_max_threads() -> io::Result<Threads> {
        let n_threads = available_parallelism()?;
        Ok(Threads::new(n_threads))
    }
    pub(crate) fn in_parallel<'a>(&self, f: &'a (dyn Fn() + Sync)) {
        let job_count = JobCount::new();
        for s in &self.senders {
            let job = Job { f, _g: job_count.inc() };
            s.send(unsafe { job.erase_lifetime() }).unwrap()
        }
    }
}

impl Drop for Threads {
    fn drop(&mut self) {
        self.senders.clear();
        for h in self.handles.drain(..) {
            let _ = h.join();
        }
    }
}

struct Job<'a> {
    f: &'a (dyn Fn() + Sync),
    _g: JobGuard<'a>,
}

struct JobCount {
    mux: Mutex<usize>,
    cv: Condvar,
}

struct JobGuard<'a> {
    count: &'a JobCount,
}

impl<'a> Job<'a> {
    unsafe fn erase_lifetime(self) -> Job<'static> {
        std::mem::transmute(self)
    }
}

impl JobCount {
    fn new() -> JobCount {
        JobCount { mux: Mutex::new(0), cv: Condvar::new() }
    }
    fn inc(&self) -> JobGuard<'_> {
        *self.mux.lock().unwrap() += 1;
        JobGuard { count: self }
    }
    fn dec(&self) {
        let mut g = self.mux.lock().unwrap();
        *g -= 1;
        if *g == 0 {
            self.cv.notify_all()
        }
    }
}

impl Drop for JobCount {
    fn drop(&mut self) {
        let mut g = self.mux.lock().unwrap();
        while *g > 0 {
            g = self.cv.wait(g).unwrap();
        }
    }
}

impl<'a> Drop for JobGuard<'a> {
    fn drop(&mut self) {
        self.count.dec()
    }
}
