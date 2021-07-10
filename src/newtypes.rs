use derive_more::{Deref, DerefMut, From, Into};
use evmap::ShallowCopy;
use std::mem::ManuallyDrop;
use std::net;

#[derive(Deref, DerefMut, Debug, Clone, Copy, PartialEq, Eq, Hash, From, Into)]
pub struct SocketAddr(net::SocketAddr);
impl ShallowCopy for SocketAddr {
    unsafe fn shallow_copy(&self) -> ManuallyDrop<Self> {
        ManuallyDrop::new(*self)
    }
}
