use std::{
    collections::BTreeSet,
    future::{poll_fn, Future},
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
    time::{Duration, Instant},
};

static mut TIMERS: Vec<(Instant, Waker)> = Vec::new();

fn main() {
    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_millis(1));
        unsafe {
            TIMERS.retain(|(deadline, waker)| {
                if Instant::now() > *deadline {
                    waker.wake_by_ref();
                    return false;
                }
                return true;
            })
        };
    });
    // tasks to wake
    static mut TTW: BTreeSet<usize> = BTreeSet::new();

    static VTABLE: RawWakerVTable = RawWakerVTable::new(
        |waker| RawWaker::new(waker, &VTABLE),
        |data| {
            let index = unsafe { data as usize };
            unsafe {
                TTW.insert(index);
            }
        },
        |data| {
            let index = unsafe { data as usize };
            unsafe {
                TTW.insert(index);
            }
        },
        |_| (),
    );
    let mut task_fut = core::pin::pin!(task());
    let mut task2_fut = core::pin::pin!(task2());

    let mut tasks: [Pin<&mut dyn Future<Output = ()>>; 2] = [task_fut, task2_fut];
    unsafe {
        TTW.insert(0);
        TTW.insert(1);
    }

    loop {
        while let Some(task_id) = unsafe { TTW.pop_first() } {
            let waker = unsafe { Waker::new(task_id as *const (), &VTABLE) };
            let mut context = Context::from_waker(&waker);

            let _ = tasks[task_id].as_mut().poll(&mut context);
        }
    }
}

async fn task() {
    loop {
        sleep(Duration::from_secs(1)).await;
        println!("task");
    }
}

async fn task2() {
    loop {
        sleep(Duration::from_secs(2)).await;
        println!("task2");
    }
}

fn sleep(duration: Duration) -> impl Future<Output = ()> {
    let start = Instant::now();
    let deadline = start + duration;
    let mut registered = false;
    poll_fn(move |_ctx| {
        if !registered {
            unsafe { TIMERS.push((deadline, _ctx.waker().clone())) };
            registered = true;
        }
        if Instant::now() > deadline {
            return Poll::Ready(());
        }
        return Poll::Pending;
    })
}
