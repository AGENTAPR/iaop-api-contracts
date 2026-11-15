# OAuth Client Configuration (realm iaop-dev)

| Client | Type | Purpose |
|---|---|---|
| iaop-developer-portal | confidential | Portal SSO; redirect `https://8083-agq5lpbszgbva3ygddcbarv6ea.instabox.tech/auth/callback` |
| iaop-scalar-dev | public + PKCE S256 | Swagger UI backend testing; redirect `https://8083-agq5lpbszgbva3ygddcbarv6ea.instabox.tech/swagger-ui/oauth2-redirect.html` |
| iaop-agent-bff | confidential | BFF sessions; redirects incl. `https://8083-agq5lpbszgbva3ygddcbarv6ea.instabox.tech/agent-bff/auth/callback` |
| iaop-*-api | audience clients | Domain token audiences |

Client scopes `aud-agent`/`aud-agency`/`aud-eligibility` carry audience mappers → optional scopes of iaop-scalar-dev.
