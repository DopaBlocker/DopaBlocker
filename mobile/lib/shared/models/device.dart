/// Espelha shared/src/models.rs → struct Device.
class Device {
  final String id;
  final String userId;
  final String deviceName;
  final String platform; // "android" | "windows"
  final bool isChild;
  final String createdAt;

  const Device({
    required this.id,
    required this.userId,
    required this.deviceName,
    required this.platform,
    required this.isChild,
    required this.createdAt,
  });

  factory Device.fromJson(Map<String, dynamic> j) => Device(
        id: j['id'] as String,
        userId: j['user_id'] as String,
        deviceName: j['device_name'] as String,
        platform: j['platform'] as String,
        isChild: j['is_child'] == true || j['is_child'] == 1,
        createdAt: j['created_at'] as String,
      );

  Map<String, dynamic> toJson() => {
        'id': id,
        'user_id': userId,
        'device_name': deviceName,
        'platform': platform,
        'is_child': isChild,
        'created_at': createdAt,
      };

  Device copyWith({
    String? id,
    String? userId,
    String? deviceName,
    String? platform,
    bool? isChild,
    String? createdAt,
  }) =>
      Device(
        id: id ?? this.id,
        userId: userId ?? this.userId,
        deviceName: deviceName ?? this.deviceName,
        platform: platform ?? this.platform,
        isChild: isChild ?? this.isChild,
        createdAt: createdAt ?? this.createdAt,
      );
}
