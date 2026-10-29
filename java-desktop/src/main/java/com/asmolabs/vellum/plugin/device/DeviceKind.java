package com.asmolabs.vellum.plugin.device;

public enum DeviceKind {
    KOBO("kobo", "Kobo"),
    KINDLE("kindle", "Kindle"),
    GENERIC("generic", "Liseuse générique");

    private final String id;
    private final String displayName;

    DeviceKind(String id, String displayName) {
        this.id = id;
        this.displayName = displayName;
    }

    public String getId() {
        return id;
    }

    public String getDisplayName() {
        return displayName;
    }

    public static DeviceKind fromString(String val) {
        if (val == null) return GENERIC;
        for (DeviceKind k : values()) {
            if (k.id.equalsIgnoreCase(val) || k.name().equalsIgnoreCase(val)) {
                return k;
            }
        }
        return GENERIC;
    }
}
