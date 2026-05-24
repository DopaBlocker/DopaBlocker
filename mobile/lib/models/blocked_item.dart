/// Espelha shared/src/models.rs → struct BlockedItem.
class BlockedItem {
  final String id;
  final String userId;
  final String itemType; // "domain" | "app" | "keyword"
  final String value;
  final bool isActive;
  final String createdAt;

  const BlockedItem({
    required this.id,
    required this.userId,
    required this.itemType,
    required this.value,
    required this.isActive,
    required this.createdAt,
  });

  factory BlockedItem.fromJson(Map<String, dynamic> j) => BlockedItem(
        id: j['id'] as String,
        userId: j['user_id'] as String,
        itemType: j['item_type'] as String,
        value: j['value'] as String,
        isActive: j['is_active'] == true || j['is_active'] == 1,
        createdAt: j['created_at'] as String,
      );

  Map<String, dynamic> toJson() => {
        'id': id,
        'user_id': userId,
        'item_type': itemType,
        'value': value,
        'is_active': isActive,
        'created_at': createdAt,
      };

  BlockedItem copyWith({
    String? id,
    String? userId,
    String? itemType,
    String? value,
    bool? isActive,
    String? createdAt,
  }) =>
      BlockedItem(
        id: id ?? this.id,
        userId: userId ?? this.userId,
        itemType: itemType ?? this.itemType,
        value: value ?? this.value,
        isActive: isActive ?? this.isActive,
        createdAt: createdAt ?? this.createdAt,
      );
}
