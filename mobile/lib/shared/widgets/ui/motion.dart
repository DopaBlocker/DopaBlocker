import 'package:flutter/material.dart';

import 'package:dopablocker_mobile/shared/theme.dart';

/// Entrada com stagger (fade + leve translate) para itens de lista. O atraso
/// cresce com [index] (até um teto) para o efeito cascata. Respeita
/// reduced-motion (mostra direto, sem animação).
class StaggeredItem extends StatefulWidget {
  final int index;
  final Widget child;
  const StaggeredItem({required this.index, required this.child, super.key});

  @override
  State<StaggeredItem> createState() => _StaggeredItemState();
}

class _StaggeredItemState extends State<StaggeredItem>
    with SingleTickerProviderStateMixin {
  late final AnimationController _c =
      AnimationController(vsync: this, duration: AppDurations.enter);

  @override
  void initState() {
    super.initState();
    final delayMs = (widget.index.clamp(0, 12)) * 40;
    Future.delayed(Duration(milliseconds: delayMs), () {
      if (mounted) _c.forward();
    });
  }

  @override
  void dispose() {
    _c.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final reduce = MediaQuery.maybeOf(context)?.disableAnimations ?? false;
    if (reduce) return widget.child;
    final curved = CurvedAnimation(parent: _c, curve: AppCurves.out);
    return FadeTransition(
      opacity: curved,
      child: SlideTransition(
        position: Tween(begin: const Offset(0, 0.06), end: Offset.zero).animate(curved),
        child: widget.child,
      ),
    );
  }
}
