// src/suspend.rs
// SIGSTOP временно отключён — замораживает весь процесс включая ksni D-Bus тред,
// из-за чего snixembed теряет иконку и трей перестаёт работать.
// Используем только hide() — ~47MB vs 100MB экономия не критична пока трей не починен.

pub fn maybe_run_as_watcher(_args: &[String]) -> bool { false }

pub fn suspend_self() {
    eprintln!("[suspend] SIGSTOP disabled — using hide() only");
}
