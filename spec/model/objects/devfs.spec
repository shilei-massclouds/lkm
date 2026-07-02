/*
 * Minimal devfs model.
 *
 * This slice mounts a devfs instance on /dev after the initial ramfs-backed
 * rootfs exists and after hwrng/block registries are ready. It creates named
 * VFS device nodes for the current hwrng surface and the default block device
 * surface. Character/block file operation dispatch, miscdevice registration,
 * devtmpfs kernel thread behaviour, uevents, sysfs links, permissions and
 * userspace device management stay deferred.
 */

predicate devfs_initialized<T>(devfs: T) -> bool;
predicate devfs_mount_point_bound<T, D>(devfs: T, dentry: D) -> bool;
predicate devfs_root_dentry_bound<T, D>(devfs: T, dentry: D) -> bool;
predicate devfs_hwrng_node_bound<T, H>(devfs: T, hwrng_core: H) -> bool;
predicate devfs_block_node_bound<T, B>(devfs: T, block_registry: B) -> bool;
predicate devfs_hwrng_node_listed<T>(devfs: T) -> bool;
predicate devfs_block_node_listed<T>(devfs: T) -> bool;
predicate devfs_device_file_ops_deferred<T>(devfs: T) -> bool;
predicate devfs_uevent_deferred<T>(devfs: T) -> bool;
predicate devfs_sysfs_deferred<T>(devfs: T) -> bool;

object DevFs: ResourceObject {
    initial_state: State::Base;

    state State::Base {
        transitions {
            on Transition::Setup -> State::Ready {
                depends_on {
                    VfsCore.state == State::Ready;
                    FsStruct.state == State::Ready;
                    rootfs_mount_created(VfsCore);
                    fs_struct_root_dentry_set(FsStruct, Dentry);
                    HwRngCore.state == State::Ready;
                    hwrng_core_current_slot_ready(HwRngCore);
                    BlockDeviceRegistry.state == State::Ready;
                    block_core_default_device_slot_ready(BlockDeviceRegistry);
                }

                drives {
                    VfsCore.Action::CreateDirectory;
                    VfsCore.Action::MountDevFsAt(Dentry);
                    VfsCore.Action::CreateDeviceNode;
                    VfsCore.Action::CreateDeviceNode;
                }

                ensures {
                    devfs_initialized(DevFs);
                    devfs_mount_created(VfsCore);
                    devfs_mount_point_bound(DevFs, Dentry);
                    devfs_root_dentry_bound(DevFs, Dentry);
                    devfs_hwrng_node_bound(DevFs, HwRngCore);
                    devfs_block_node_bound(DevFs, BlockDeviceRegistry);
                    devfs_hwrng_node_listed(DevFs);
                    devfs_block_node_listed(DevFs);
                    devfs_device_file_ops_deferred(DevFs);
                    devfs_uevent_deferred(DevFs);
                    devfs_sysfs_deferred(DevFs);
                }
            }
        }
    }

    state State::Ready {
        invariant {
            devfs_initialized(DevFs);
            devfs_mount_created(VfsCore);
            devfs_hwrng_node_bound(DevFs, HwRngCore);
            devfs_block_node_bound(DevFs, BlockDeviceRegistry);
            devfs_hwrng_node_listed(DevFs);
            devfs_block_node_listed(DevFs);
            devfs_device_file_ops_deferred(DevFs);
            devfs_uevent_deferred(DevFs);
            devfs_sysfs_deferred(DevFs);
        }
    }
}
