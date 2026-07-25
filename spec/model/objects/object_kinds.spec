/*
 * Common object kind type shells.
 *
 * These types document the reusable object categories used by object
 * declarations. Most existing phase files also use these names as object kind
 * labels; declaring them here makes later common type definitions able to
 * extend the same vocabulary explicitly.
 */

type PhaseObject {
}

type SystemObject {
}

type ComputerObject: SystemObject {
}

type PlatformObject: SystemObject {
}

type FirmwareObject: SystemObject {
}

type PrepareObject {
}

type IsaObject {
}

type KernelObject: SystemObject {
}

type DeviceObject {
}

type MemoryObject {
}

type ResourceObject {
}

type FlowObject {
}

type CPUObject {
}
