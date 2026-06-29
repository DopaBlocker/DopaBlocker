import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:dopablocker_mobile/core/api/api_base.dart';
import 'package:dopablocker_mobile/core/api/api_dtos.dart';
import 'package:dopablocker_mobile/shared/models/blocked_item.dart';

final blocklistApiProvider =
    Provider<BlocklistApi>((ref) => BlocklistApi(ref.read(dioProvider)));

/// Cliente das rotas de blocklist (`/blocklist*`): CRUD de itens bloqueados,
/// leitura com ETag/304 (poll do filho/pessoal) e filtro adulto.
class BlocklistApi {
  final Dio _dio;
  BlocklistApi(this._dio);

  /// Último ETag visto de `GET /blocklist` (B2). Usado para `If-None-Match`.
  String? _blocklistEtag;

  Future<List<BlockedItem>> getBlocklist() async {
    final res = await _dio.get('/blocklist');
    _blocklistEtag = res.headers.value('etag') ?? _blocklistEtag;
    return (res.data as List).map((e) => BlockedItem.fromJson(e as Map<String, dynamic>)).toList();
  }

  /// Busca a blocklist usando ETag/`If-None-Match` (B2 — poll periódico do
  /// filho). Retorna `null` quando o backend responde **304** (lista
  /// inalterada): o chamador mantém a lista atual e poupa banda. Erros de auth
  /// (401, revogação) continuam propagando como `DioException`.
  Future<List<BlockedItem>?> getBlocklistIfChanged() async {
    final res = await _dio.get(
      '/blocklist',
      options: Options(
        headers: _blocklistEtag != null ? {'If-None-Match': _blocklistEtag} : null,
        validateStatus: (s) => s == 200 || s == 304,
      ),
    );
    if (res.statusCode == 304) return null;
    _blocklistEtag = res.headers.value('etag') ?? _blocklistEtag;
    return (res.data as List).map((e) => BlockedItem.fromJson(e as Map<String, dynamic>)).toList();
  }

  Future<BlockedItem> addBlockedItem({required String itemType, required String value}) async {
    final res = await _dio.post('/blocklist', data: {'item_type': itemType, 'value': value});
    return BlockedItem.fromJson(res.data as Map<String, dynamic>);
  }

  Future<void> removeBlockedItem(String id) => _dio.delete('/blocklist/$id');

  Future<AdultFilterSettings> setAdultFilter(bool enabled) async {
    final res = await _dio.put('/blocklist/adult-filter', data: {'enabled': enabled});
    return AdultFilterSettings.fromJson(res.data as Map<String, dynamic>);
  }
}
