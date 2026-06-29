import 'package:dio/dio.dart';
import 'package:firebase_auth/firebase_auth.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:dopablocker_mobile/core/api/api_exception.dart';
import 'package:dopablocker_mobile/core/api/auth_api.dart';
import 'package:dopablocker_mobile/features/auth/providers/auth_provider.dart';
import 'package:dopablocker_mobile/shared/theme.dart';
import 'package:dopablocker_mobile/shared/widgets/ui/ui_kit.dart';

/// Tela de login e cadastro para modos Pessoal e Pais.
/// Recebe `arguments` da rota como String: 'personal' ou 'parental'.
///
/// Fluxo de cadastro com email/senha:
///   1. POST /auth/email-code/start  → código enviado por email
///   2. POST /auth/email-code/verify → retorna emailVerificationToken
///   3. Firebase createAccount       → cria conta no Firebase
///   4. POST /auth/register          → cria conta local com o token
///
/// O estado [AuthPendingLocalRegistration] é usado quando o Firebase signup
/// ocorreu mas o /auth/register ainda não foi chamado.
class LoginScreen extends ConsumerStatefulWidget {
  const LoginScreen({super.key});

  @override
  ConsumerState<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends ConsumerState<LoginScreen>
    with SingleTickerProviderStateMixin {
  late final TabController _tabs;
  final _emailCtrl = TextEditingController();
  final _passwordCtrl = TextEditingController();
  final _nameCtrl = TextEditingController();
  final _codeCtrl = TextEditingController();
  final _formKey = GlobalKey<FormState>();
  bool _loading = false;
  String? _error;

  // Cadastro por email: passa a exibir o campo de código após o envio.
  bool _codeSent = false;
  // Google: marca uma tentativa ativa de cadastro/login para concluir o
  // registro de conta nova (AuthPendingLocalRegistration) sem disparar no boot.
  bool _completingGoogle = false;
  bool _googleRegistering = false;

  @override
  void initState() {
    super.initState();
    _tabs = TabController(length: 2, vsync: this);
  }

  @override
  void dispose() {
    _tabs.dispose();
    _emailCtrl.dispose();
    _passwordCtrl.dispose();
    _nameCtrl.dispose();
    _codeCtrl.dispose();
    super.dispose();
  }

  String get _mode =>
      (ModalRoute.of(context)?.settings.arguments as String?) ?? 'personal';

  void _setError(String? msg) => setState(() => _error = msg);

  Future<void> _login() async {
    if (!_formKey.currentState!.validate()) return;
    setState(() { _loading = true; _error = null; });
    try {
      await ref.read(authProvider.notifier).loginWithEmail(
            _emailCtrl.text.trim(),
            _passwordCtrl.text,
          );
    } on FirebaseAuthException catch (e) {
      _setError(_firebaseMessage(e.code));
    } catch (_) {
      _setError('Não foi possível conectar. Verifique sua conexão.');
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _loginWithGoogle() async {
    setState(() { _loading = true; _error = null; _completingGoogle = true; });
    try {
      await ref.read(authProvider.notifier).loginWithGoogle();
      // Se a conta já existir, o estado vira AuthAuthenticated e o app navega.
      // Se for conta nova, vira AuthPendingLocalRegistration e o listener do
      // build conclui o cadastro.
    } on FirebaseAuthException catch (e) {
      _completingGoogle = false;
      _setError(_firebaseMessage(e.code));
    } catch (_) {
      _completingGoogle = false;
      _setError('Login com Google falhou.');
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  /// Conclui o cadastro de conta NOVA via Google (estado pendente). Backend não
  /// exige token de email para provider federado.
  Future<void> _completeGoogleRegistration(User user) async {
    setState(() { _loading = true; _error = null; });
    try {
      final name = (user.displayName?.trim().isNotEmpty ?? false)
          ? user.displayName!.trim()
          : (user.email?.split('@').first ?? 'Usuário');
      await ref.read(authProvider.notifier).register(displayName: name, mode: _mode);
    } catch (_) {
      _setError('Não foi possível concluir o cadastro com Google.');
    } finally {
      _googleRegistering = false;
      _completingGoogle = false;
      if (mounted) setState(() => _loading = false);
    }
  }

  /// Cadastro com email/senha — passo 1: dispara o envio do código por email.
  Future<void> _startEmailRegistration() async {
    final email = _emailCtrl.text.trim();
    if (_nameCtrl.text.trim().isEmpty) { _setError('Informe seu nome.'); return; }
    if (!email.contains('@')) { _setError('Email inválido.'); return; }
    if (_passwordCtrl.text.length < 6) { _setError('Senha: mínimo 6 caracteres.'); return; }
    setState(() { _loading = true; _error = null; });
    try {
      await ref.read(authApiProvider).emailCodeStart(email);
      setState(() => _codeSent = true);
    } on DioException catch (e) {
      final err = e.error;
      _setError(err is ApiException ? err.message : 'Não foi possível enviar o código.');
    } catch (_) {
      _setError('Não foi possível enviar o código.');
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  /// Cadastro com email/senha — passo 2: verifica o código, cria no Firebase e
  /// conclui o registro local com o token.
  Future<void> _confirmEmailRegistration() async {
    final email = _emailCtrl.text.trim();
    final code = _codeCtrl.text.trim();
    if (code.length < 6) { _setError('Informe o código de 6 dígitos.'); return; }
    setState(() { _loading = true; _error = null; });
    try {
      final verify = await ref.read(authApiProvider).emailCodeVerify(email, code);
      await ref.read(authProvider.notifier).registerWithEmail(
            email: email,
            password: _passwordCtrl.text,
            displayName: _nameCtrl.text.trim(),
            mode: _mode,
            emailVerificationToken: verify.emailVerificationToken,
          );
      // Sucesso → authProvider emite AuthAuthenticated → app navega.
    } on FirebaseAuthException catch (e) {
      _setError(_firebaseMessage(e.code));
    } on DioException catch (e) {
      final err = e.error;
      _setError(err is ApiException ? err.message : 'Cadastro falhou. Tente novamente.');
    } catch (_) {
      _setError('Cadastro falhou. Tente novamente.');
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  /// Conclui o cadastro a partir do estado pendente (sessão Firebase já existe).
  /// Com o backend idempotente + reclaim, isto cria a conta nova OU recupera uma
  /// conta órfã do mesmo email. Para provider `password` exige o token de email.
  Future<void> _finishPending({String? token}) async {
    setState(() { _loading = true; _error = null; });
    try {
      final auth = ref.read(authProvider);
      if (auth is! AuthPendingLocalRegistration) return;
      final fbUser = auth.firebaseUser;
      final name = (fbUser.displayName?.trim().isNotEmpty ?? false)
          ? fbUser.displayName!.trim()
          : (fbUser.email?.split('@').first ?? 'Usuário');
      await ref.read(authProvider.notifier).register(
            displayName: name,
            mode: _mode,
            emailVerificationToken: token,
          );
    } on DioException catch (e) {
      final err = e.error;
      _setError(err is ApiException ? err.message : 'Não foi possível concluir o cadastro.');
    } catch (_) {
      _setError('Não foi possível concluir o cadastro.');
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  /// Provider `password` no estado pendente: dispara o envio do código de email
  /// (prova de posse exigida pelo backend para concluir/recuperar a conta).
  Future<void> _sendPendingCode(String email) async {
    setState(() { _loading = true; _error = null; });
    try {
      await ref.read(authApiProvider).emailCodeStart(email);
      setState(() => _codeSent = true);
    } on DioException catch (e) {
      final err = e.error;
      _setError(err is ApiException ? err.message : 'Não foi possível enviar o código.');
    } catch (_) {
      _setError('Não foi possível enviar o código.');
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _verifyPendingAndFinish(String email) async {
    final code = _codeCtrl.text.trim();
    if (code.length < 6) { _setError('Informe o código de 6 dígitos.'); return; }
    setState(() { _loading = true; _error = null; });
    try {
      final verify = await ref.read(authApiProvider).emailCodeVerify(email, code);
      await _finishPending(token: verify.emailVerificationToken);
    } on DioException catch (e) {
      final err = e.error;
      _setError(err is ApiException ? err.message : 'Código inválido.');
      if (mounted) setState(() => _loading = false);
    } catch (_) {
      _setError('Código inválido.');
      if (mounted) setState(() => _loading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    // Conta nova via Google: ao virar pendente durante uma tentativa ativa,
    // conclui o cadastro local (uma única vez).
    ref.listen<AuthState>(authProvider, (_, next) {
      if (next is AuthPendingLocalRegistration &&
          _completingGoogle &&
          !_googleRegistering) {
        _googleRegistering = true;
        _completeGoogleRegistration(next.firebaseUser);
      }
    });

    // Estado pendente (Firebase ok, falta concluir/recuperar a conta local).
    // Painel dedicado, sem abas — paridade com a tela-curativo do desktop.
    final authState = ref.watch(authProvider);
    if (authState is AuthPendingLocalRegistration) {
      return _buildPendingScaffold(authState);
    }

    final modeLabel = _mode == 'parental' ? 'Pais' : 'Pessoal';
    return Scaffold(
      appBar: AppBar(
        title: Text('DopaBlocker — $modeLabel'),
        bottom: TabBar(
          controller: _tabs,
          labelColor: AppColors.textPrimary,
          unselectedLabelColor: AppColors.textSecondary,
          indicatorColor: AppColors.primary,
          tabs: const [Tab(text: 'Entrar'), Tab(text: 'Cadastrar')],
        ),
      ),
      body: TabBarView(
        controller: _tabs,
        children: [
          _buildLoginTab(),
          _buildRegisterTab(),
        ],
      ),
    );
  }

  /// Painel da tela-curativo (estado pendente). Sem abas: só conclui/recupera a
  /// conta local. Para Google (provider verificado) basta um toque; para
  /// `password` roda o código de email como prova de posse.
  Widget _buildPendingScaffold(AuthPendingLocalRegistration pending) {
    final fbUser = pending.firebaseUser;
    final email = fbUser.email ?? '';
    final isPassword =
        fbUser.providerData.any((p) => p.providerId == 'password');

    return Scaffold(
      appBar: AppBar(title: const Text('DopaBlocker')),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(AppSpacing.x6),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const SizedBox(height: AppSpacing.x4),
            Text('Finalizando seu cadastro', style: AppType.h2),
            const SizedBox(height: AppSpacing.x2),
            Text(
              'Sua sessão está pronta. Falta concluir o cadastro neste dispositivo.',
              style: AppType.bodySm.copyWith(color: AppColors.textSecondary),
            ),
            const SizedBox(height: AppSpacing.x6),
            if (isPassword && _codeSent) ...[
              Text('Enviamos um código de 6 dígitos para $email.',
                  style: AppType.bodySm),
              const SizedBox(height: AppSpacing.x4),
              TextField(
                controller: _codeCtrl,
                decoration: _dec('Código de verificação'),
                keyboardType: TextInputType.number,
                maxLength: 6,
                style: AppType.body,
              ),
              _errorBox(),
              const SizedBox(height: AppSpacing.x4),
              AppButton(
                label: 'Concluir cadastro',
                loading: _loading,
                onPressed: () => _verifyPendingAndFinish(email),
              ),
            ] else if (isPassword) ...[
              _errorBox(),
              AppButton(
                label: 'Enviar código para concluir',
                loading: _loading,
                onPressed: () => _sendPendingCode(email),
              ),
            ] else ...[
              _errorBox(),
              AppButton(
                label: 'Concluir cadastro',
                loading: _loading,
                onPressed: () => _finishPending(),
              ),
            ],
            const SizedBox(height: AppSpacing.x3),
            AppButton(
              label: 'Entrar com outra conta',
              variant: AppButtonVariant.secondary,
              onPressed:
                  _loading ? null : () => ref.read(authProvider.notifier).logout(),
            ),
          ],
        ),
      ),
    );
  }

  /// Decoração padrão dos campos — herda o inputDecorationTheme (preenchido,
  /// cantos arredondados, foco roxo); só define o label.
  InputDecoration _dec(String label) => InputDecoration(labelText: label);

  Widget _errorBox() {
    if (_error == null) return const SizedBox.shrink();
    return Padding(
      padding: const EdgeInsets.only(top: AppSpacing.x3),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Icon(Icons.error_outline, size: 16, color: AppColors.danger),
          const SizedBox(width: AppSpacing.x2),
          Expanded(
            child: Text(_error!,
                style: AppType.bodySm.copyWith(color: AppColors.danger)),
          ),
        ],
      ),
    );
  }

  Widget _buildLoginTab() {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(AppSpacing.x6),
      child: Form(
        key: _formKey,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const SizedBox(height: AppSpacing.x4),
            TextFormField(
              controller: _emailCtrl,
              decoration: _dec('Email'),
              keyboardType: TextInputType.emailAddress,
              style: AppType.body,
              validator: (v) => (v == null || !v.contains('@')) ? 'Email inválido' : null,
            ),
            const SizedBox(height: AppSpacing.x4),
            TextFormField(
              controller: _passwordCtrl,
              decoration: _dec('Senha'),
              obscureText: true,
              style: AppType.body,
              validator: (v) => (v == null || v.length < 6) ? 'Mínimo 6 caracteres' : null,
            ),
            _errorBox(),
            const SizedBox(height: AppSpacing.x6),
            AppButton(label: 'Entrar', onPressed: _login, loading: _loading),
            const SizedBox(height: AppSpacing.x3),
            AppButton(
              label: 'Entrar com Google',
              variant: AppButtonVariant.secondary,
              icon: Icons.login,
              onPressed: _loading ? null : _loginWithGoogle,
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildRegisterTab() {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(AppSpacing.x6),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: _codeSent ? _registerStepCode() : _registerStepFields(),
      ),
    );
  }

  /// Passo 1 do cadastro: nome/email/senha + envio do código.
  List<Widget> _registerStepFields() => [
        const SizedBox(height: AppSpacing.x4),
        TextField(
          controller: _nameCtrl,
          decoration: _dec('Nome'),
          style: AppType.body,
        ),
        const SizedBox(height: AppSpacing.x4),
        TextField(
          controller: _emailCtrl,
          decoration: _dec('Email'),
          keyboardType: TextInputType.emailAddress,
          style: AppType.body,
        ),
        const SizedBox(height: AppSpacing.x4),
        TextField(
          controller: _passwordCtrl,
          decoration: _dec('Senha'),
          obscureText: true,
          style: AppType.body,
        ),
        _errorBox(),
        const SizedBox(height: AppSpacing.x2),
        Text(
          'Um código de verificação será enviado para seu email antes de criar a conta.',
          style: AppType.caption,
        ),
        const SizedBox(height: AppSpacing.x6),
        AppButton(label: 'Criar conta', onPressed: _startEmailRegistration, loading: _loading),
        const SizedBox(height: AppSpacing.x3),
        AppButton(
          label: 'Cadastrar com Google',
          variant: AppButtonVariant.secondary,
          icon: Icons.login,
          onPressed: _loading ? null : _loginWithGoogle,
        ),
      ];

  /// Passo 2 do cadastro: digitar o código recebido por email.
  List<Widget> _registerStepCode() => [
        const SizedBox(height: AppSpacing.x4),
        Text(
          'Enviamos um código de 6 dígitos para ${_emailCtrl.text.trim()}.',
          style: AppType.bodySm,
        ),
        const SizedBox(height: AppSpacing.x4),
        TextField(
          controller: _codeCtrl,
          decoration: _dec('Código de verificação'),
          keyboardType: TextInputType.number,
          maxLength: 6,
          style: AppType.body,
        ),
        _errorBox(),
        const SizedBox(height: AppSpacing.x6),
        AppButton(
          label: 'Confirmar e criar conta',
          onPressed: _confirmEmailRegistration,
          loading: _loading,
        ),
        const SizedBox(height: AppSpacing.x3),
        AppButton(
          label: 'Voltar',
          variant: AppButtonVariant.secondary,
          onPressed: _loading
              ? null
              : () => setState(() {
                    _codeSent = false;
                    _codeCtrl.clear();
                    _error = null;
                  }),
        ),
      ];

  String _firebaseMessage(String code) => switch (code) {
        'user-not-found' || 'wrong-password' || 'invalid-credential' =>
          'Email ou senha incorretos.',
        'email-already-in-use' => 'Email já está em uso.',
        'weak-password' => 'Senha muito fraca.',
        'network-request-failed' => 'Sem conexão com a internet.',
        'sign_in_cancelled' => 'Login cancelado.',
        _ => 'Erro de autenticação ($code).',
      };
}
