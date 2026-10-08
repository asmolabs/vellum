package com.asmolabs.vellum.plugin;

public interface VellumPlugin {

    PluginDescriptor getDescriptor();

    default String getId() {
        return getDescriptor().id();
    }

    default String getName() {
        return getDescriptor().name();
    }

    default boolean isExternal() {
        return getDescriptor().external();
    }
}
