use core::cell::{RefCell, UnsafeCell};
use core::mem::forget;
use core::ptr::NonNull;

use fast_trap::{FlowContext, FreeTrapStack};

use crate::trap::fast_handler;
use crate::{constants::LEN_STACK_PER_HART, Supervisor};

static mut ROOT_STACK: Stack = Stack::ZERO;

pub(crate) fn prepare_for_trap() {
    unsafe { ROOT_STACK.load_as_stack() };
}

/// 获取此 hart 的 local hsm 对象。
pub(crate) fn local_hsm() -> &'static HsmCell<Supervisor> {
    unsafe { &ROOT_STACK.hart_context().hsm }
}

struct Stack([u8; LEN_STACK_PER_HART]);

impl Stack {
    /// 零初始化以避免加载。
    const ZERO: Self = Self([0; LEN_STACK_PER_HART]);

    /// 从栈上取出硬件线程状态。
    #[inline]
    fn hart_context(&mut self) -> &mut HartContext {
        unsafe { &mut *self.0.as_mut_ptr().cast() }
    }

    fn load_as_stack(&'static mut self) {
        let hart = self.hart_context();
        let context_ptr = hart.context_ptr();
        hart.init();
        let range = self.0.as_ptr_range();
        forget(
            FreeTrapStack::new(
                range.start as usize..range.end as usize,
                |_| {},
                context_ptr,
                fast_handler,
            )
            .unwrap()
            .load(),
        );
    }
}

#[allow(unused)]
#[derive(Clone, Copy)]
pub enum HartState {
    Started,
    Stopped,
    StartPending,
    StopPending,
    Suspended,
    SuspendPending,
    ResumePending,
}

pub(crate) struct HsmCell<T> {
    status: RefCell<HartState>,
    inner: UnsafeCell<Option<T>>,
}

impl<T> HsmCell<T> {
    fn new() -> Self {
        Self {
            status: RefCell::new(HartState::Stopped),
            inner: UnsafeCell::new(None),
        }
    }

    pub fn start(&self) -> Result<T, HartState> {
        let status = *self.status.borrow();
        match status {
            HartState::StartPending => {
                *self.status.borrow_mut() = HartState::Started;
                Ok(unsafe { self.inner.get().as_mut().unwrap() }
                    .take()
                    .unwrap())
            }
            _ => Err(*self.status.borrow()),
        }
    }

    pub fn prepare(&self, v: T) {
        *self.status.borrow_mut() = HartState::StartPending;
        unsafe { self.inner.get().as_mut().unwrap() }.replace(v);
    }
}

struct HartContext {
    trap_context: FlowContext,
    hsm: HsmCell<Supervisor>,
}

impl HartContext {
    #[inline]
    fn init(&mut self) {
        self.hsm = HsmCell::new();
    }

    #[inline]
    fn context_ptr(&mut self) -> NonNull<FlowContext> {
        unsafe { NonNull::new_unchecked(&mut self.trap_context) }
    }
}
