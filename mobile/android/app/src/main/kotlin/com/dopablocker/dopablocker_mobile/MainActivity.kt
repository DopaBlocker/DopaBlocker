package com.dopablocker.dopablocker_mobile

import android.content.Intent
import com.dopablocker.dopablocker_mobile.channel.BlockingChannelHandler
import com.dopablocker.dopablocker_mobile.channel.BlockingMethods
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

/// Ponto de entrada Android. Registra o MethodChannel de bloqueio
/// (`BlockingMethods.CHANNEL`, espelha lib/core/channels/blocking_channel.dart) e
/// delega toda a lógica para [BlockingChannelHandler]. O `onActivityResult` é
/// repassado ao handler para concluir o consentimento de VPN.
class MainActivity : FlutterActivity() {

    private lateinit var channelHandler: BlockingChannelHandler

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        channelHandler = BlockingChannelHandler(this)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, BlockingMethods.CHANNEL)
            .setMethodCallHandler(channelHandler)
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        channelHandler.onVpnConsentResult(requestCode, resultCode)
    }
}
