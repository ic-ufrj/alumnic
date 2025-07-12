Para testar em seu computador, crie um túnel SSH para a porta do LDAP:

    ssh -L 9090:146.164.41.21:389 operador@petropolis.dcc.ufrj.br -p 22022

Agora, no terminal em que for executar o programa:

    export LDAP_URL='ldap://localhost:9090'
    export LDAP_BIND_DN='cn=admin,dc=dcc,dc=ufrj,dc=br'
    export LDAP_BIND_PW='SENHA_DO_LDAP'

## TODOs

- [ ] Decidir quantos caracteres uma senha deve ter e devidamente alterar todos
      os "6 a 12 caracteres" na documentação

