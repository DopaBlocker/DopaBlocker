import { api } from './api';
import type { Device } from '../types';

const DEVICE_KEY_PREFIX = 'dopablocker:owner-device:';

export async function ensureOwnerDeviceRegistered(userId: string): Promise<Device | null> {
    const key = DEVICE_KEY_PREFIX + userId;
    const cachedId = localStorage.getItem(key);

    const devices = await api.listDevices();
    const cached = cachedId
        ? devices.find((device) => device.id === cachedId && !device.is_child)
        : null;
    if (cached) return cached;

    const existing = devices.find(
        (device) => !device.is_child && device.platform === 'windows',
    );
    if (existing) {
        localStorage.setItem(key, existing.id);
        return existing;
    }

    const created = await api.registerDevice({
        device_name: 'DopaBlocker Desktop',
        platform: 'windows',
    });
    localStorage.setItem(key, created.id);
    return created;
}
