/// Espelha shared/src/models.rs → struct User.
class User {
  final String id;
  final String firebaseUid;
  final String email;
  final String displayName;
  final String mode; // "personal" | "parental"
  final String createdAt;

  const User({
    required this.id,
    required this.firebaseUid,
    required this.email,
    required this.displayName,
    required this.mode,
    required this.createdAt,
  });

  factory User.fromJson(Map<String, dynamic> j) => User(
        id: j['id'] as String,
        firebaseUid: j['firebase_uid'] as String,
        email: j['email'] as String,
        displayName: j['display_name'] as String,
        mode: j['mode'] as String,
        createdAt: j['created_at'] as String,
      );

  Map<String, dynamic> toJson() => {
        'id': id,
        'firebase_uid': firebaseUid,
        'email': email,
        'display_name': displayName,
        'mode': mode,
        'created_at': createdAt,
      };

  User copyWith({
    String? id,
    String? firebaseUid,
    String? email,
    String? displayName,
    String? mode,
    String? createdAt,
  }) =>
      User(
        id: id ?? this.id,
        firebaseUid: firebaseUid ?? this.firebaseUid,
        email: email ?? this.email,
        displayName: displayName ?? this.displayName,
        mode: mode ?? this.mode,
        createdAt: createdAt ?? this.createdAt,
      );

  bool get isParental => mode == 'parental';
  bool get isPersonal => mode == 'personal';
}
