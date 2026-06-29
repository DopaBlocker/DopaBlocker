import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:dopablocker_mobile/core/api/api_exception.dart';
import 'package:dopablocker_mobile/core/api/blocklist_api.dart';
import 'package:dopablocker_mobile/core/channels/blocking_channel.dart';
import 'package:dopablocker_mobile/shared/models/blocked_item.dart';
import 'package:dopablocker_mobile/features/auth/providers/auth_provider.dart';

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
  (ref) => BlockingNotifier(ref.read(blocklistApiProvider), ref)..load(),
);

class BlockingNotifier extends StateNotifier<BlockingState> {
  final BlocklistApi _api;
  final Ref _ref;

  /// Timer do poll periódico da blocklist (B2). Ativo no filho (edições do pai)
  /// e na conta pessoal/pai (mudanças feitas em outro device da mesma conta).
  Timer? _pollTimer;

  /// Intervalo do poll. ~45s é aceitável para sync entre devices e barato
  /// graças ao ETag/304 do backend.
  static const Duration _pollInterval = Duration(seconds: 45);

  BlockingNotifier(this._api, this._ref)
      : super(BlockingState(isLoading: true, activeSince: _startOfToday())) {
    // Recarrega e (re)inicia o poll quando a sessão muda (login, troca de conta,
    // vínculo de filho). Sem isto, o provider só carregaria na criação e o
    // device pessoal/pai nunca pegaria mudanças feitas em outro device.
    _ref.listen<AuthState>(authProvider, (_, next) => _onAuthChanged(next));
  }

  void _onAuthChanged(AuthState next) {
    if (next is AuthAuthenticated || next is AuthChildSession) {
      load();
    } else {
      _pollTimer?.cancel();
    }
  }

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
    _startPollIfNeeded();
  }

  /// Liga o poll periódico para sessões que precisam receber mudanças vindas de
  /// OUTRO device: filho vinculado (edições do pai) e conta pessoal/pai
  /// (mudanças em outro device da mesma conta). Idempotente.
  void _startPollIfNeeded() {
    final auth = _ref.read(authProvider);
    _pollTimer?.cancel();
    if (auth is! AuthChildSession && auth is! AuthAuthenticated) return;
    _pollTimer = Timer.periodic(_pollInterval, (_) => _pollBlocklist());
  }

  Future<void> _pollBlocklist() async {
    try {
      final updated = await _api.getBlocklistIfChanged();
      if (updated != null) {
        state = state.copyWith(items: updated);
        _syncNative();
      }
    } on DioException catch (e) {
      final err = e.error;
      if (err is ApiException && err.statusCode == 401) {
        // Só o filho desloga em 401 (Device Token revogado pelo pai → o layout
        // redireciona). Para Firebase, o refresh do token é feito no
        // AuthInterceptor (core/api); um 401 aqui não deve derrubar a sessão.
        if (_ref.read(authProvider) is AuthChildSession) {
          await _ref.read(authProvider.notifier).logout();
        }
      }
    } catch (_) {/* rede/5xx: tenta de novo no próximo tick */}
  }

  @override
  void dispose() {
    _pollTimer?.cancel();
    super.dispose();
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

  Future<void> toggleAdultFilter(bool enabled) async {
    state = state.copyWith(isAdultFilterEnabled: enabled);
    // Aplica no engine nativo (troca o resolver upstream — C4).
    BlockingChannel.setAdultFilter(enabled).catchError((_) {});
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

  /// Garante que o engine do filho está rodando: sobe a VPN se ainda não estiver
  /// ativa e empurra a blocklist para o nativo. Idempotente — chamado ao entrar
  /// na sessão de filho e a cada retorno ao app (depois que as permissões são
  /// concedidas). É o que faz o bloqueio definido pelo pai realmente valer no
  /// dispositivo do filho (antes disto, a lista era empurrada mas a VPN nunca
  /// subia sozinha).
  Future<void> ensureEngineRunning() async {
    try {
      final active = await BlockingChannel.isVpnActive();
      if (!active) {
        await BlockingChannel.startVpn();
      }
    } catch (_) {/* nativo indisponível */}
    _syncNative();
  }

  /// Repassa a blocklist de domínios e apps ativos para o serviço nativo, mais
  /// o estado do filtro adulto.
  ///
  /// Regra do pai imune: no device do pai em modo parental, envia listas vazias
  /// (o pai não bloqueia a si mesmo). Conta pessoal e device do filho aplicam a
  /// lista normalmente.
  void _syncNative() {
    if (!state.isBlockingActive) return;
    final auth = _ref.read(authProvider);
    final isParentDevice = auth is AuthAuthenticated && auth.user.isParental;

    final domains = isParentDevice
        ? const <String>[]
        : state.items
            .where((i) => i.isActive && i.itemType == 'domain')
            .map((i) => i.value)
            .toList();
    final apps = isParentDevice
        ? const <String>[]
        : state.items
            .where((i) => i.isActive && i.itemType == 'app')
            .map((i) => i.value)
            .toList();

    BlockingChannel.updateBlocklist(domains).catchError((_) {});
    BlockingChannel.updateBlockedApps(apps).catchError((_) {});
    // No device do pai (imune) o filtro adulto também não se aplica.
    BlockingChannel.setAdultFilter(isParentDevice ? false : state.isAdultFilterEnabled)
        .catchError((_) {});
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
