# Payload coding constraints

Payload preparation executes inside the already-Online KernelInitFlow. It prepares the stable
KernelInitUserAppRuntime and an ApplicationInstance. The commit boundary replaces only the Runtime's
application reference, then enters user mode using the context already bound to KernelInitTask.

Failed exec leaves the old ApplicationInstance and every outer identity unchanged. Successful repeated
exec preserves KernelInitTask, KernelInitFlow and UserAppRuntime identity. No Flow lifecycle transition is
part of exec.
