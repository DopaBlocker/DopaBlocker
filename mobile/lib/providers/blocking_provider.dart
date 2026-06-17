import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../channels/blocking_channel.dart';
import '../core/api_client.dart';
import '../models/blocked_item.dart';

/// Estado da blocklist e do bloqueio ativo.
class BlockingState {
  final List<BlockedItem> items;
  final bool isBlockingActive;
  final bool isAdultFilterEnabled;
  final DateTime? activeSince;
  final bool isLoading;

  const BlockingState({
    this.items = const [],
    this.isBlockingActive = true,
    this.isAdultFilterEnabled = true,
    this.activeSince,
    this.isLoading = false,
  });

  int get siteCount =>
      items.where((i) => i.isActive && i.itemType != 'app').length;
  int get appCount => items.where((i) => i.isActive && i.itemType == 'app').length;
  int get activeCount => items.where((i) => i.isActive).length;

  BlockingState copyWith({
    List<BlockedItem>? items,
    bool? isBlockingActive,
    bool? isAdultFilterEnabled,
    DateTime? activeSince,
    bool? isLoading,
  }) =>
      BlockingState(
        items: items ?? this.items,
        isBlockingActive: isBlockingActive ?? this.isBlockingActive,
        isAdultFilterEnabled: isAdultFilterEnabled ?? this.isAdultFilterEnabled,
        activeSince: activeSince ?? this.activeSince,
        isLoading: isLoading ?? this.isLoading,
      );
}

final blockingProvider = StateNotifierProvider<BlockingNotifier, BlockingState>(
  (ref) => BlockingNotifier(ref.read(apiClientProvider))..load(),
);

class BlockingNotifier extends StateNotifier<BlockingState> {
  final ApiClient _api;

  BlockingNotifier(this._api)
      : super(BlockingState(isLoading: true, activeSince: _startOfToday()));

  static DateTime _startOfToday() {
    final now = DateTime.now();
    return DateTime(now.year, now.month, now.day);
  }

  Future<void> load() async {
    try {
      final items = await _api.getBlocklist();
      state = state.copyWith(items: items, isLoading: false);
    } catch (_) {
      // Backend indisponível (preview/offline): popula com dados
      // representativos do mockup para o app continuar navegável.
      state = state.copyWith(items: _demoItems(), isLoading: false);
    }
    _syncNative();
  }

  Future<void> addItem(String value, String itemType) async {
    try {
      final created = await _api.addBlockedItem(itemType: itemType, value: value);
      state = state.copyWith(items: [...state.items, created]);
    } catch (_) {
      // Offline: cria localmente para refletir na UI.
      state = state.copyWith(items: [
        ...state.items,
        BlockedItem(
          id: 'local-${DateTime.now().microsecondsSinceEpoch}',
          userId: '',
          itemType: itemType,
          value: value,
          isActive: true,
          createdAt: DateTime.now().toIso8601String(),
        ),
      ]);
    }
    _syncNative();
  }

  Future<void> removeItem(String id) async {
    state = state.copyWith(items: state.items.where((i) => i.id != id).toList());
    try {
      await _api.removeBlockedItem(id);
    } catch (_) {/* já removido localmente */}
    _syncNative();
  }

  void toggleItemActive(BlockedItem item) {
    state = state.copyWith(items: [
      for (final i in state.items)
        if (i.id == item.id) i.copyWith(isActive: !i.isActive) else i,
    ]);
    _syncNative();
  }

  Future<void> toggleAdultFilter(bool enabled) async {
    state = state.copyWith(isAdultFilterEnabled: enabled);
    try {
      await _api.setAdultFilter(enabled);
    } catch (_) {/* sincroniza quando voltar online */}
  }

  Future<void> toggleBlocking(bool active) async {
    state = state.copyWith(
      isBlockingActive: active,
      activeSince: active ? DateTime.now() : null,
    );
    try {
      if (active) {
        await BlockingChannel.startVpn();
      } else {
        await BlockingChannel.stopVpn();
      }
    } catch (_) {/* serviço nativo ainda stub (Fase M2) ou indisponível */}
    _syncNative();
  }

  /// Repassa a blocklist de domínios ativos para o serviço nativo de VPN.
  void _syncNative() {
    if (!state.isBlockingActive) return;
    final domains = state.items
        .where((i) => i.isActive && i.itemType == 'domain')
        .map((i) => i.value)
        .toList();
    BlockingChannel.updateBlocklist(domains).catchError((_) {});
  }

  List<BlockedItem> _demoItems() {
    final now = DateTime.now().toIso8601String();
    return [
      BlockedItem(id: 'demo-1', userId: '', itemType: 'domain', value: 'instagram.com', isActive: true, createdAt: now),
      BlockedItem(id: 'demo-2', userId: '', itemType: 'domain', value: 'x.com', isActive: true, createdAt: now),
      BlockedItem(id: 'demo-3', userId: '', itemType: 'domain', value: 'youtube.com', isActive: false, createdAt: now),
      BlockedItem(id: 'demo-4', userId: '', itemType: 'keyword', value: 'apostas', isActive: true, createdAt: now),
    ];
  }
}
