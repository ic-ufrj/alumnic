Para testar em seu computador, crie um túnel SSH para a porta do LDAP:

    ssh -L 9090:146.164.41.21:389 operador@petropolis.dcc.ufrj.br -p 22022

Na sua configuração `~/.config/alumnic/config.yaml`:

    ldap_url: "ldap://127.0.0.1:9090"
    ldap_bind_dn: "cn=admin,dc=dcc,dc=ufrj,dc=br"
    ldap_bind_pw: "SENHA DO LDAP"

## TODOs

- [ ] Decidir quantos caracteres uma senha deve ter e devidamente alterar todos
      os "6 a 12 caracteres" na documentação

