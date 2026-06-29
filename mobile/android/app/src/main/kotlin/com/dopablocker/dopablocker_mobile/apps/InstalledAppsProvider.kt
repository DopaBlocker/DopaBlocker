package com.dopablocker.dopablocker_mobile.apps

import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.drawable.BitmapDrawable
import android.graphics.drawable.Drawable
import android.util.Base64
import java.io.ByteArrayOutputStream

/// Enumera os apps lançáveis instalados para o seletor visual de bloqueio de
/// app (Flutter). Devolve nome, package e ícone PNG em base64. Exclui o próprio
/// DopaBlocker. A visibilidade depende do bloco <queries> MAIN/LAUNCHER no
/// AndroidManifest (Android 11+/API 30).
object InstalledAppsProvider {

    private const val ICON_SIZE_PX = 96

    /// Lista os apps com ícone no launcher, ordenados por nome. Cada item é um
    /// mapa serializável pelo MethodChannel: `packageName`, `appName`, `icon`
    /// (base64, opcional). Pode ser pesado — chame fora da main thread.
    fun listLaunchableApps(context: Context): List<Map<String, String>> {
        val pm = context.packageManager
        val intent = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER)
        val resolved = pm.queryIntentActivities(intent, 0)

        // De-dup por package (um package pode ter mais de uma activity launcher).
        val seen = HashSet<String>()
        val apps = ArrayList<Map<String, String>>(resolved.size)
        for (info in resolved) {
            val pkg = info.activityInfo?.packageName ?: continue
            if (pkg == context.packageName) continue
            if (!seen.add(pkg)) continue
            val label = info.loadLabel(pm)?.toString() ?: pkg
            val icon = runCatching { encodeIcon(info.loadIcon(pm)) }.getOrNull()
            apps.add(
                buildMap {
                    put("packageName", pkg)
                    put("appName", label)
                    if (icon != null) put("icon", icon)
                },
            )
        }
        apps.sortBy { (it["appName"] ?: "").lowercase() }
        return apps
    }

    private fun encodeIcon(drawable: Drawable): String {
        val out = ByteArrayOutputStream()
        drawableToBitmap(drawable).compress(Bitmap.CompressFormat.PNG, 100, out)
        return Base64.encodeToString(out.toByteArray(), Base64.NO_WRAP)
    }

    private fun drawableToBitmap(drawable: Drawable): Bitmap {
        if (drawable is BitmapDrawable && drawable.bitmap != null) {
            return Bitmap.createScaledBitmap(drawable.bitmap, ICON_SIZE_PX, ICON_SIZE_PX, true)
        }
        val bitmap = Bitmap.createBitmap(ICON_SIZE_PX, ICON_SIZE_PX, Bitmap.Config.ARGB_8888)
        val canvas = Canvas(bitmap)
        drawable.setBounds(0, 0, canvas.width, canvas.height)
        drawable.draw(canvas)
        return bitmap
    }
}
