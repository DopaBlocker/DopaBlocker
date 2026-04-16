// =============================================================================
// auth_service — reservado para lógica de autenticação fora do middleware.
// =============================================================================
// Hoje este módulo está INTENCIONALMENTE vazio. A validação de Firebase JWT
// e Device Token vive em `middleware.rs`, e o CRUD de users vive em
// `user_service.rs`. Essa separação dá:
//
//   - middleware.rs: acesso ao `State` da app; aplicado a todas rotas
//     protegidas de uma vez.
//   - user_service.rs: queries SQL puras sobre `users`, reutilizáveis por
//     qualquer handler.
//
// Casos futuros que morariam AQUI (não em nenhum dos outros dois):
//
//   - Fluxo de "linkar conta Google a conta email" do Firebase (quando o
//     mesmo Firebase UID aparece com provedores diferentes).
//   - Geração de tokens internos custom (se um dia precisarmos de algo
//     além de Firebase JWT e Device Token).
//   - Rate limiting específico de endpoints de auth (tentativas de login).
//   - Rotação/revogação em lote de Device Tokens.
//
// Enquanto não há nenhum desses, o arquivo fica como marcador da existência
// dessa fronteira conceitual — para que a próxima pessoa saiba onde colocar.
// =============================================================================
