# IAOP Route-to-Specification Parity Report

**Date:** 2026-11-14  
**Audit Scope:** Implemented Microservices & Agent BFF  

---

## 1. Summary of Parity Audits

| Service Repository | Commit SHA | Registered Runtime Routes | Documented OpenAPI Operations | Parity Status |
| :--- | :---: | :---: | :---: | :--- |
| **`iaop-agent-bff`** | `c5a2056` | 52 | 52 | **EXACT_MATCH (100%)** |
| **`iaop-agent-service`** | `0dfcd70` | 17 | 17 | **EXACT_MATCH (100%)** |
| **`iaop-agency-service`** | `e3f703e` | 3 | 3 | **EXACT_MATCH (100%)** |
| **Remaining Domain Services** | *Various* | Baseline / Scaffold | Draft / Proposed | **PROPOSED_CONTRACT** |

---

## 2. Detailed Route Parity: `iaop-agent-bff` (52 Operations)

| # | Runtime Method | Runtime Path | OpenAPI Operation ID | Tag | Match Status |
| :---: | :---: | :--- | :--- | :--- | :---: |
| 1 | `GET` | `/api/v1/me` | `getMe` | Identity & Workspace | `EXACT_MATCH` |
| 2 | `GET` | `/api/v1/workspaces` | `listWorkspaces` | Identity & Workspace | `EXACT_MATCH` |
| 3 | `POST` | `/api/v1/workspaces/select` | `selectWorkspace` | Identity & Workspace | `EXACT_MATCH` |
| 4 | `GET` | `/api/v1/preferences` | `getPreferences` | Identity & Workspace | `EXACT_MATCH` |
| 5 | `PATCH` | `/api/v1/preferences` | `updatePreferences` | Identity & Workspace | `EXACT_MATCH` |
| 6 | `GET` | `/api/v1/me/today` | `getTodayDashboard` | Today Dashboard | `EXACT_MATCH` |
| 7 | `GET` | `/api/v1/tasks` | `listTasks` | Today Dashboard | `EXACT_MATCH` |
| 8 | `PATCH` | `/api/v1/tasks/{id}` | `patchTask` | Today Dashboard | `EXACT_MATCH` |
| 9 | `GET` | `/api/v1/agenda` | `getAgenda` | Today Dashboard | `EXACT_MATCH` |
| 10 | `GET` | `/api/v1/search` | `globalSearch` | Utilities & Support | `EXACT_MATCH` |
| 11 | `GET` | `/api/v1/notifications` | `listNotifications` | Utilities & Support | `EXACT_MATCH` |
| 12 | `POST` | `/api/v1/notifications/{id}/read` | `markNotificationRead` | Utilities & Support | `EXACT_MATCH` |
| 13 | `GET` | `/api/v1/support/tickets` | `listSupportTickets` | Utilities & Support | `EXACT_MATCH` |
| 14 | `POST` | `/api/v1/support/tickets` | `createSupportTicket` | Utilities & Support | `EXACT_MATCH` |
| 15 | `GET` | `/api/v1/recruitment/candidates` | `listCandidates` | Recruitment | `EXACT_MATCH` |
| 16 | `POST` | `/api/v1/recruitment/candidates` | `createCandidate` | Recruitment | `EXACT_MATCH` |
| 17 | `GET` | `/api/v1/recruitment/my-referrals` | `getMyReferrals` | Recruitment | `EXACT_MATCH` |
| 18 | `GET` | `/api/v1/onboarding/me` | `getOnboardingMe` | Onboarding | `EXACT_MATCH` |
| 19 | `POST` | `/api/v1/onboarding/tasks/{id}/submit` | `submitOnboardingTask` | Onboarding | `EXACT_MATCH` |
| 20 | `POST` | `/api/v1/documents/upload-intent` | `createUploadIntent` | Documents | `EXACT_MATCH` |
| 21 | `POST` | `/api/v1/documents/complete` | `completeDocument` | Documents | `EXACT_MATCH` |
| 22 | `GET` | `/api/v1/documents/{id}/download-intent` | `getDownloadIntent` | Documents | `EXACT_MATCH` |
| 23 | `GET` | `/api/v1/learning/courses` | `listCourses` | Learning | `EXACT_MATCH` |
| 24 | `POST` | `/api/v1/learning/courses/{id}/enroll` | `enrollCourse` | Learning | `EXACT_MATCH` |
| 25 | `GET` | `/api/v1/learning/my-progress` | `getMyLearningProgress` | Learning | `EXACT_MATCH` |
| 26 | `GET` | `/api/v1/certifications/me` | `getCertificationsMe` | Certification | `EXACT_MATCH` |
| 27 | `POST` | `/api/v1/certifications/evidence` | `submitCertificationEvidence` | Certification | `EXACT_MATCH` |
| 28 | `GET` | `/api/v1/agency/hierarchy` | `getAgencyHierarchy` | Agency | `EXACT_MATCH` |
| 29 | `GET` | `/api/v1/agency/my-team` | `getMyTeam` | Agency | `EXACT_MATCH` |
| 30 | `GET` | `/api/v1/customers` | `listCustomers` | Customers | `EXACT_MATCH` |
| 31 | `POST` | `/api/v1/customers` | `createCustomer` | Customers | `EXACT_MATCH` |
| 32 | `GET` | `/api/v1/customers/{id}` | `getCustomerDetail` | Customers | `EXACT_MATCH` |
| 33 | `GET` | `/api/v1/products` | `listProducts` | Sales & Products | `EXACT_MATCH` |
| 34 | `POST` | `/api/v1/illustrations/calculate` | `calculateIllustration` | Sales & Products | `EXACT_MATCH` |
| 35 | `POST` | `/api/v1/proposals` | `createProposal` | Sales & Products | `EXACT_MATCH` |
| 36 | `GET` | `/api/v1/proposals` | `listProposals` | Sales & Products | `EXACT_MATCH` |
| 37 | `GET` | `/api/v1/policies` | `listPolicies` | Policies | `EXACT_MATCH` |
| 38 | `GET` | `/api/v1/policies/{id}` | `getPolicyDetail` | Policies | `EXACT_MATCH` |
| 39 | `GET` | `/api/v1/claims` | `listClaims` | Claims Assistance | `EXACT_MATCH` |
| 40 | `POST` | `/api/v1/claims` | `createClaim` | Claims Assistance | `EXACT_MATCH` |
| 41 | `GET` | `/api/v1/claims/{id}` | `getClaimDetail` | Claims Assistance | `EXACT_MATCH` |
| 42 | `GET` | `/api/v1/earnings/summary` | `getEarningsSummary` | Earnings | `EXACT_MATCH` |
| 43 | `GET` | `/api/v1/earnings/statements` | `listEarningsStatements` | Earnings | `EXACT_MATCH` |
| 44 | `GET` | `/api/v1/performance/metrics` | `getPerformanceMetrics` | Performance | `EXACT_MATCH` |
| 45 | `GET` | `/api/v1/performance/rankings` | `getPerformanceRankings` | Performance | `EXACT_MATCH` |
| 46 | `GET` | `/api/v1/operations/{id}` | `getOperationStatus` | Operations & Status | `EXACT_MATCH` |
| 47 | `GET` | `/api/v1/me/offboarding-status` | `getOffboardingStatus` | Operations & Status | `EXACT_MATCH` |
| 48 | `GET` | `/api/v1/me/restrictions` | `getRestrictions` | Operations & Status | `EXACT_MATCH` |
| 49 | `GET` | `/api/v1/auth/login` | `authLogin` | Authentication & Session | `EXACT_MATCH` |
| 50 | `GET` | `/api/v1/auth/callback` | `authCallback` | Authentication & Session | `EXACT_MATCH` |
| 51 | `POST` | `/api/v1/auth/logout` | `authLogout` | Authentication & Session | `EXACT_MATCH` |
| 52 | `GET` | `/api/v1/auth/session` | `authSession` | Authentication & Session | `EXACT_MATCH` |

---

## 3. Detailed Route Parity: `iaop-agent-service` (17 Operations)

| # | Runtime Method | Runtime Path | OpenAPI Operation ID | Match Status |
| :---: | :---: | :--- | :--- | :---: |
| 1 | `GET` | `/health/live` | `live` | `EXACT_MATCH` |
| 2 | `GET` | `/health/ready` | `ready` | `EXACT_MATCH` |
| 3 | `GET` | `/api/v1/system/info` | `system_info` | `EXACT_MATCH` |
| 4 | `POST` | `/api/v1/agents` | `Agent:register` | `EXACT_MATCH` |
| 5 | `GET` | `/api/v1/agents` | `Agent:list` | `EXACT_MATCH` |
| 6 | `GET` | `/api/v1/agents/{id}` | `Agent:getById` | `EXACT_MATCH` |
| 7 | `GET` | `/api/v1/agents/code/{code}` | `Agent:getByCode` | `EXACT_MATCH` |
| 8 | `POST` | `/api/v1/agents/{id}/submit-for-activation` | `Agent:submitForActivation` | `EXACT_MATCH` |
| 9 | `POST` | `/api/v1/agents/{id}/activate` | `Agent:activate` | `EXACT_MATCH` |
| 10 | `POST` | `/api/v1/agents/{id}/suspend` | `Agent:suspend` | `EXACT_MATCH` |
| 11 | `POST` | `/api/v1/agents/{id}/reinstate` | `Agent:reinstate` | `EXACT_MATCH` |
| 12 | `POST` | `/api/v1/agents/{id}/terminate` | `Agent:terminate` | `EXACT_MATCH` |
| 13 | `GET` | `/api/v1/agents/{id}/lifecycle-history` | `Agent:history:read` | `EXACT_MATCH` |
| 14 | `GET` | `/api/v1/agents/{id}/appointments` | `Agent:getAppointments` | `EXACT_MATCH` |
| 15 | `GET` | `/api/v1/agents/{id}/identity-links` | `Agent:getIdentityLinks` | `EXACT_MATCH` |
| 16 | `POST` | `/api/v1/agents/{id}/identity-links` | `Agent:linkIdentity` | `EXACT_MATCH` |
| 17 | `DELETE`| `/api/v1/agents/{id}/identity-links/{link_id}` | `Agent:unlinkIdentity` | `EXACT_MATCH` |
