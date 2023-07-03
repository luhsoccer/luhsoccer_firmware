use core::{cell::RefCell, future::poll_fn, task::Poll};

use defmt::Format;
use embassy_sync::{
    blocking_mutex::{raw::RawMutex, Mutex},
    waitqueue::MultiWakerRegistration,
};

#[derive(Debug, Format, PartialEq, Eq, Clone, Copy)]
pub enum Error {
    SubscriberLimit,
}

pub struct Observable<M: RawMutex, T, const SUBS: usize> {
    inner: Mutex<M, RefCell<ObservableState<T, SUBS>>>,
}

impl<M: RawMutex, T, const SUBS: usize> Observable<M, T, SUBS> {
    pub const fn new(value: T) -> Self {
        Self {
            inner: Mutex::new(RefCell::new(ObservableState::new(value))),
        }
    }

    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.inner.lock(|cell| {
            let inner = cell.borrow();
            inner.value.clone()
        })
    }

    pub fn set(&self, value: T) {
        self.inner.lock(|cell| {
            let mut inner = cell.borrow_mut();
            inner.value = value;
            inner.id += 1;
            inner.wakers.wake();
        })
    }

    pub fn set_if_different(&self, value: T)
    where
        T: PartialEq,
    {
        self.inner.lock(|cell| {
            let mut inner = cell.borrow_mut();
            if inner.value != value {
                inner.value = value;
                inner.id += 1;
                inner.wakers.wake();
            }
        })
    }

    pub fn subscriber(&self) -> Result<Subscriber<'_, M, T, SUBS>, Error> {
        self.inner.lock(|cell| {
            let mut inner = cell.borrow_mut();
            if inner.subs >= SUBS {
                return Err(Error::SubscriberLimit);
            }
            inner.subs += 1;
            Ok(())
        })?;
        Ok(Subscriber {
            sub_var: self,
            last_id: 0, // Always initialize the last id to 0 so the current value is received
                        // once.
        })
    }
}

struct ObservableState<T, const SUBS: usize> {
    value: T,
    wakers: MultiWakerRegistration<SUBS>,
    id: u64,
    subs: usize,
}

impl<T, const SUBS: usize> ObservableState<T, SUBS> {
    const fn new(value: T) -> Self {
        Self {
            value,
            wakers: MultiWakerRegistration::new(),
            id: 1,
            subs: 0,
        }
    }
}

pub struct Subscriber<'s, M: RawMutex, T, const SUBS: usize> {
    sub_var: &'s Observable<M, T, SUBS>,
    last_id: u64,
}

impl<M: RawMutex, T: Clone, const SUBS: usize> Subscriber<'_, M, T, SUBS> {
    pub fn get(&mut self) -> T {
        self.sub_var.inner.lock(|cell| {
            let inner = cell.borrow();
            self.last_id = inner.id;
            inner.value.clone()
        })
    }

    pub async fn next_value(&mut self) -> T {
        poll_fn(|cx| {
            self.sub_var.inner.lock(|cell| {
                let mut inner = cell.borrow_mut();
                if self.last_id < inner.id {
                    self.last_id = inner.id;
                    Poll::Ready(inner.value.clone())
                } else {
                    inner.wakers.register(cx.waker());
                    Poll::Pending
                }
            })
        })
        .await
    }
}

impl<M: RawMutex, T, const SUBS: usize> Drop for Subscriber<'_, M, T, SUBS> {
    fn drop(&mut self) {
        self.sub_var.inner.lock(|cell| {
            let mut inner = cell.borrow_mut();
            inner.subs -= 1;
        })
    }
}
