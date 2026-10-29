package com.asmolabs.vellum.plugin.device;

import java.io.File;

public record ConnectedDevice(
        DeviceKind kind,
        String name,
        File mountPoint,
        int bookCount
) {}
