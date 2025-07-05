//! Synchronous subscription implementation

use std::marker::PhantomData;
use std::time::Duration;

use crate::transport::InternalSubscription;
use crate::Error;

/// Synchronous subscription for streaming data
pub struct Subscription<T> {
    inner: InternalSubscription,
    _phantom: PhantomData<T>,
}

impl<T> Subscription<T> {
    pub fn new(inner: InternalSubscription) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }

    pub fn next(&self) -> Option<Result<T, Error>> {
        todo!("Implement next")
    }

    pub fn try_next(&self) -> Option<Result<T, Error>> {
        todo!("Implement try_next")
    }

    pub fn next_timeout(&self, _timeout: Duration) -> Option<Result<T, Error>> {
        todo!("Implement next_timeout")
    }
}

impl<T> Iterator for Subscription<T> {
    type Item = Result<T, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!("Implement iterator next")
    }
}