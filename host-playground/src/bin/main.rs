ue core::{
    future::Future,
    pin::{pin, Pin},
    task::{Context, RawWaker, RawWakerVTable, Waker},
};
use std::{
    sync::atomic::{AtomicU32, Ordering}, task::Poll, time::{Duration, Instant}
};

fn main() {
    let task1 = pin!(task1());
    let task2 = pin!(task2());
    run(&mut [task1, task2]);
}

async fn task1(waiter: &mut Waiter) {
    // event generator
    loop {
        waiter.send();
        sleep(Duration::from_millis(100)).await; 
        println!("task1")
    }
}

async fn task2() {
    loop {
        waiter.await;
        println!("task2")
    }
}

static NOOP_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    |data| RawWaker::new(data, &NOOP_WAKER_VTABLE),
    |_| {},
    |_| {},
    |_| {},
);

static WAKE_MASK: AtomicU32 = AtomicU32::new(0);

static LILOS_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    |data| RawWaker::new(data, &NOOP_WAKER_VTABLE),
    |data| {
        let task_id = data as u32;
        WAKE_MASK.fetch_or(1 << task_id, Ordering::Relaxed);
    },
    |data| {
        let task_id = data as u32;
        WAKE_MASK.fetch_or(1 << task_id, Ordering::Relaxed);
    },
    |_| {},
);

fn run(tasks: &mut [Pin<&mut dyn Future<Output = ()>>]) {
    let waker = unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &NOOP_WAKER_VTABLE)) };
    let mut context = Context::from_waker(&waker);
    let mut last_instant = Instant::now();
    loop {
        if Instant::now() - last_instant > Duration::from_millis(1){
            for task in tasks.iter_mut() {
                task.as_mut().poll(&mut context);
            }
            last_instant = Instant::now();
        }
    }
}

async fn sleep(duration: Duration) {
    let start = Instant::now();
    while Instant::now() < (start + duration) {
        Yield { polled: false }.await
    }
}

struct Yield {
    polled: bool,
}

impl Future for Yield {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
        if self.polled {
            return Poll::Ready(());
        }
        self.as_mut().polled = true;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

struct Waiter {
    polled: bool,
}

impl Future for Waiter {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
        if self.polled {
            return Poll::Ready(());
        }
        self.as_mut().polled = true;
        Poll::Pending
    }
}
