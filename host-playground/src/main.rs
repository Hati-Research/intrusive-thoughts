use core::{
    future::Future,
    pin::{pin, Pin},
    task::{Context, RawWaker, RawWakerVTable, Waker},
};
use std::{
    convert::Infallible,
    task::Poll,
    time::{Duration, Instant},
};

fn main() {
    let task1 = pin!(task1());
    let task2 = pin!(task2());
    run(&mut [task1, task2]);
}

async fn task1() {
    loop {
        sleep(Duration::from_secs(1)).await;
        println!("task1")
    }
}

async fn task2() {
    loop {
        sleep(Duration::from_secs(3)).await;
        println!("task2")
    }
}

static NOOP_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    |data| RawWaker::new(data, &NOOP_WAKER_VTABLE),
    |_| {},
    |_| {},
    |_| {},
);

fn run(tasks: &mut [Pin<&mut dyn Future<Output = ()>>]) {
    let waker = unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &NOOP_WAKER_VTABLE)) };
    let mut context = Context::from_waker(&waker);
    loop {
        for task in tasks.iter_mut() {
            task.as_mut().poll(&mut context);
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
        Poll::Pending
    }
}
