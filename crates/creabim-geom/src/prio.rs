//! Thread priorities (Windows): the UI thread a step above normal, the
//! workers that build drawings and scenes a step below, so a busy machine
//! (a compile, a screen recording, the workers themselves on every core)
//! still leaves the window a core to answer the pointer with. On a Mac
//! the same through quality-of-service classes: the UI thread is
//! user-interactive, the workers user-initiated, so on Apple Silicon both
//! stay on the performance cores (utility or background work would be
//! moved to the efficiency cores). Elsewhere, nothing changes.

#[cfg(windows)]
mod win {
    #[link(name = "kernel32")]
    extern "system" {
        pub fn GetCurrentThread() -> isize;
        pub fn SetThreadPriority(thread: isize, priority: i32) -> i32;
    }
}

#[cfg(windows)]
fn set(priority: i32) {
    // SAFETY: the pseudo handle of the calling thread, always valid
    unsafe {
        win::SetThreadPriority(win::GetCurrentThread(), priority);
    }
}

#[cfg(target_os = "macos")]
mod mac {
    pub const QOS_CLASS_USER_INTERACTIVE: u32 = 0x21;
    pub const QOS_CLASS_USER_INITIATED: u32 = 0x19;
    extern "C" {
        pub fn pthread_set_qos_class_self_np(qos_class: u32, relative_priority: i32) -> i32;
    }
}

#[cfg(target_os = "macos")]
fn qos(class: u32) {
    // SAFETY: changes only the calling thread's own class
    unsafe {
        mac::pthread_set_qos_class_self_np(class, 0);
    }
}

/// The calling thread draws the window: a step above normal.
pub fn ui() {
    #[cfg(windows)]
    set(1); // THREAD_PRIORITY_ABOVE_NORMAL
    #[cfg(target_os = "macos")]
    qos(mac::QOS_CLASS_USER_INTERACTIVE);
}

/// The calling thread builds in the background: a step below normal.
pub fn background() {
    #[cfg(windows)]
    set(-1); // THREAD_PRIORITY_BELOW_NORMAL
    #[cfg(target_os = "macos")]
    qos(mac::QOS_CLASS_USER_INITIATED);
}
