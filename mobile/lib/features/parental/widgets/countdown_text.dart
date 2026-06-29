import 'dart:async';

import 'package:flutter/material.dart';

/// Texto que conta regressivamente até [expiresAt], formatado como mm:ss.
/// Exibe [expiredLabel] quando o tempo acaba.
class CountdownText extends StatefulWidget {
  final DateTime expiresAt;
  final TextStyle? style;
  final String expiredLabel;

  const CountdownText({
    required this.expiresAt,
    this.style,
    this.expiredLabel = 'expirado',
    super.key,
  });

  @override
  State<CountdownText> createState() => _CountdownTextState();
}

class _CountdownTextState extends State<CountdownText> {
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    _timer = Timer.periodic(const Duration(seconds: 1), (_) {
      if (mounted) setState(() {});
    });
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final remaining = widget.expiresAt.difference(DateTime.now());
    final text = remaining.isNegative
        ? widget.expiredLabel
        : '${remaining.inMinutes.toString().padLeft(2, '0')}:'
            '${(remaining.inSeconds % 60).toString().padLeft(2, '0')}';
    return Text(text, style: widget.style);
  }
}
