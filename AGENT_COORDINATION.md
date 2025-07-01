# Agent Coordination System - LLM Archive V2

## 🎯 Mission
Build a high-performance LLM conversation archive in 2 weeks with full features, using specialized agents for parallel development.

## 👥 Agent Roster

### 1. **Coordination Agent** (COORD)
- **Role**: Project manager and inter-agent communication
- **Status**: Active
- **Current Task**: Setting up coordination system

### 2. **Requirements Agent** (REQ)
- **Role**: Extract definitive requirements from V1 codebase and docs
- **Status**: Pending launch
- **Focus**: Analyze V1 failures, define V2 success criteria

### 3. **Backend Agent** (BACK)
- **Role**: Rust backend development
- **Status**: Pending launch
- **Focus**: API, database, search implementation

### 4. **Frontend Agent** (FRONT)
- **Role**: SvelteKit UI development
- **Status**: Pending launch
- **Focus**: Modern, fast, responsive interface

### 5. **Import Agent** (IMP)
- **Role**: Optimize import pipeline
- **Status**: Pending launch
- **Focus**: Parser improvements, performance, accuracy

### 6. **Testing Agent** (TEST)
- **Role**: Comprehensive test suite
- **Status**: Pending launch
- **Focus**: Unit, integration, E2E tests

## 📋 Communication Protocol

### Memory Files
- `REQUIREMENTS.md` - Definitive feature list [TAG: REQ]
- `BACKEND_STATUS.md` - Backend progress [TAG: BACK]
- `FRONTEND_STATUS.md` - Frontend progress [TAG: FRONT]
- `IMPORT_STATUS.md` - Import optimization [TAG: IMP]
- `TEST_STATUS.md` - Test coverage [TAG: TEST]
- `DAILY_STANDUP.md` - Daily progress [TAG: DAILY]

### Sync Points
- Every 4 hours: Status update in respective files
- Daily: Standup summary in DAILY_STANDUP.md
- Blockers: Immediate update in BLOCKERS.md

## 🚀 Launch Sequence

1. Requirements Agent → Extract V1 insights (2 hours)
2. Backend + Frontend → Parallel scaffold (Day 1)
3. Import Agent → Analyze V1 parsers (Day 1)
4. Testing Agent → Set up CI/CD (Day 1)
5. All agents → Iterative development (Days 2-14)

## 📊 Success Metrics

### Week 1 Goals
- [ ] Core API running
- [ ] Basic UI scaffold
- [ ] Import pipeline working
- [ ] CI/CD operational

### Week 2 Goals
- [ ] All features implemented
- [ ] <100ms search performance
- [ ] 100% test coverage
- [ ] Production ready

## 🔄 Current Status
- Project initialized
- GitHub repo created
- Agent system activated
- Ready to launch specialized agents

---
*Last Updated: [timestamp]*
*Coordinator: COORD*