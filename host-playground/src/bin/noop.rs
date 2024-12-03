use std::{future::Future, pin::pin, sync::atomic::{AtomicBool, Ordering}, task::{Poll, RawWaker, RawWakerVTable, Waker}, thread::sleep_ms, time::Duration};

async fn task1() {
    loop {
        println!("task1");
        Delay::new().await;
    }
}

struct Delay {
    count: u32,
}

impl Delay {
    fn new() -> Self {
        Self {
            count: 0
        }
    }
}

impl Future for Delay {
    type Output = ();

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        self.as_mut().count += 1;

        if self.as_ref().count == 100 {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

static TASK_AWOKEN: AtomicBool = AtomicBool::new(false);

static NOOP_VTABLE: RawWakerVTable = RawWakerVTable::new(
    |data| RawWaker::new(data, &NOOP_VTABLE),
    |_| {},
    |_| {
        TASK_AWOKEN.store(true, Ordering::SeqCst);
    },
    |_| {},
);

static WAKER: Waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &NOOP_VTABLE)) };


fn main() {
    // init executor time driver
    std::thread::spawn(|| {
        loop {
            WAKER.wake_by_ref();
            std::thread::sleep(Duration::from_millis(10));
        }
    });

    let mut task1 = pin!(task1());

    let mut context = std::task::Context::from_waker(&WAKER);

    loop {
        if TASK_AWOKEN.swap(false, Ordering::SeqCst) {
            task1.as_mut().poll(&mut context);
        }
    }
}
