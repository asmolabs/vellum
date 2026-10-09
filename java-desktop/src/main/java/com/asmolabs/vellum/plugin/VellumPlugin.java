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

    record TestResult(boolean success, String message) {
        public static TestResult ok(String message) {
            return new TestResult(true, message);
        }
        public static TestResult error(String message) {
            return new TestResult(false, message);
        }
    }

    default TestResult testConfiguration(java.util.Map<String, Object> config) {
        return TestResult.ok("Configuration opérationnelle.");
    }
}
