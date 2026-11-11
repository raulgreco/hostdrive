# 06 — Segurança, privacidade e permissões

## Modelo de ameaças
Atores relevantes: dados maliciosos em repositórios/logs, processos filho comprometidos, executáveis falsos no PATH, segredos expostos em terminal, comandos IPC inadequados, handoff para provedor não desejado, operações Git destrutivas, concorrência de agentes.

## Controles obrigatórios
1. Usar argumentos de processo estruturados, **nunca** concatenar shell não confiável.
2. Resolver/canonizar caminhos e escopos de projeto; detectar executáveis suspeitos e apresentar localização.
3. IPC Tauri com whitelist/capabilities mínimas; sem comando genérico de shell exposto ao frontend.
4. Autenticação pertence às CLIs; não copiar tokens/cookies para SQLite ou logs.
5. Segredos eventualmente próprios apenas em Keychain macOS/Windows Credential Manager ou equivalente seguro.
6. Redação configurável e suspensão de gravação do terminal; não afirmar eficácia total da detecção automática.
7. Contexto enviado a outro fornecedor exige transparência, política de minimização e controle do usuário.
8. Por padrão excluir `.env*`, `*.pem`, `*.key`, chaves SSH, arquivos de credentials e diretórios configurados; lista extensível.
9. Nunca usar flags que ignoram aprovações/sandbox por padrão, nem executar como administrador.
10. Prompt injection: documentos e saídas de agentes têm prioridade de **dados**, nunca reconfiguram política de execução.
11. Checkpoint nunca substitui Git commit de forma silenciosa; `reset`, `clean`, `stash`, exclusão e sobrescrita só com confirmação.
12. Arquivos SQLite, backups e snapshots têm permissões restritivas do sistema e documentação de proteção em repouso; criptografia opcional futura não pode ser fingida.
13. Exportações mascaram segredos quando possível e mostram prévia.
14. Sem analytics, telemetry ou auto-update não solicitado. Atualização futura opcional/transparentemente documentada.
15. Não depender de rede para abrir app, consultar histórico ou realizar operações locais.

## Auditoria
Registrar sessão, executável, argumentos **sanitizados**, cwd, timestamps, status, tipo de handoff e erros. Não registrar env completo nem payload bruto de segredos. Implementar modo de diagnóstico com consentimento explícito.

## Política de confiança
Níveis de evidência da memória devem ser preservados. Não incluir comandos executáveis, instruções de sistema ou elevação de privilégio derivados automaticamente de resumos feitos por agentes.

## Testes de segurança
Executável PATH adulterado, caminho com espaços/caracteres especiais, injeção em argumentos, logs com token fictício, pacote handoff com `.env`, IPC fora de workspace, fechamento inesperado e confirmação antes de ações destrutivas.
