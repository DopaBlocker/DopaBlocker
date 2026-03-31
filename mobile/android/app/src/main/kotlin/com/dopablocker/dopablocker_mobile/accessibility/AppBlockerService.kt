// AccessibilityService para detectar e bloquear abertura de apps.
// Implementar: estender android.accessibilityservice.AccessibilityService,
// monitorar TYPE_WINDOW_STATE_CHANGED events, verificar package name
// contra blocklist de apps, se bloqueado exibir overlay de bloqueio
// ou redirecionar para o DopaBlocker. Requer permissão de Accessibility.

package com.dopablocker.dopablocker_mobile.accessibility
