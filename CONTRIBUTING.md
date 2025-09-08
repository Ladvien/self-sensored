# Contributing to Health Export REST API

Thank you for your interest in contributing to the Health Export REST API! This guide provides comprehensive information on how to contribute effectively to this project.

## 📋 Table of Contents

- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Code Style and Standards](#code-style-and-standards)
- [Testing Requirements](#testing-requirements)
- [Pull Request Process](#pull-request-process)
- [Documentation Standards](#documentation-standards)
- [Issue Reporting](#issue-reporting)
- [Security Guidelines](#security-guidelines)
- [Community Guidelines](#community-guidelines)

## 🚀 Getting Started

### Prerequisites

Before you start contributing, ensure you have:

- Python 3.13+ installed
- PostgreSQL 15+ with PostGIS extension
- Redis 7.0+
- Poetry for dependency management
- Git for version control
- Docker (optional, for development environment)

### Development Environment Setup

1. **Fork and Clone**
```bash
# Fork the repository on GitHub first
git clone https://github.com/your-username/self-sensored.git
cd self-sensored
```

2. **Install Dependencies**
```bash
# Install Poetry if not already installed
curl -sSL https://install.python-poetry.org | python3 -

# Install project dependencies including dev tools
poetry install --with dev

# Activate virtual environment
poetry shell
```

3. **Set Up Services**
```bash
# Option 1: Use Docker Compose (recommended)
docker-compose up -d postgres redis

# Option 2: Install services locally
# See README.md for detailed local installation instructions
```

4. **Configure Environment**
```bash
# Copy environment template
cp .env.example .env

# Edit configuration for development
nano .env
```

5. **Database Setup**
```bash
# Run database migrations
poetry run alembic upgrade head

# Optionally, load test data
poetry run python scripts/load_test_data.py
```

6. **Install Pre-commit Hooks**
```bash
# Install pre-commit hooks for code quality
poetry run pre-commit install

# Test pre-commit setup
poetry run pre-commit run --all-files
```

7. **Verify Setup**
```bash
# Run tests to verify everything works
poetry run pytest

# Start development server
poetry run uvicorn app.main:app --reload
```

## 🔄 Development Workflow

### Branch Management

We follow a simplified Git flow:

- **`master`** - Production-ready code, always deployable
- **`develop`** - Integration branch for features (if needed for complex features)
- **Feature branches** - `feature/feature-name` for new features
- **Bug fix branches** - `bugfix/issue-description` for bug fixes
- **Hotfix branches** - `hotfix/critical-issue` for production issues

### Working on Features

1. **Create Feature Branch**
```bash
# Ensure you're on master and up to date
git checkout master
git pull upstream master

# Create and switch to feature branch
git checkout -b feature/your-feature-name
```

2. **Development Cycle**
```bash
# Make changes, add tests, update documentation
# Run tests frequently
poetry run pytest

# Check code quality
poetry run flake8 app/
poetry run black --check app/
poetry run isort --check-only app/

# Run pre-commit hooks
poetry run pre-commit run --all-files
```

3. **Commit Guidelines**
```bash
# Use conventional commit format
git commit -m "feat: add API key authentication with Argon2 hashing"
git commit -m "fix: resolve connection pool exhaustion issue"
git commit -m "docs: update README with deployment instructions"
git commit -m "test: add integration tests for health data ingestion"
```

### Commit Message Format

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix  
- `docs`: Documentation only changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks, dependency updates

**Examples:**
```
feat(auth): implement API key authentication with Redis caching
fix(db): resolve connection pool exhaustion in high-load scenarios
docs(api): add comprehensive endpoint documentation with examples
test(integration): add end-to-end health data processing tests
```

## 📏 Code Style and Standards

### Python Code Style

We follow strict code formatting and linting standards:

**Required Tools:**
- **Black**: Code formatting (line length: 88)
- **isort**: Import sorting
- **flake8**: Linting with additional plugins
- **mypy**: Static type checking (optional but recommended)

**Configuration Files:**
- `pyproject.toml` - Black, isort, and tool configuration
- `.flake8` - Flake8 linting rules
- `mypy.ini` - Type checking configuration

### Code Formatting Commands

```bash
# Format code automatically
poetry run black app/ tests/
poetry run isort app/ tests/

# Check formatting (CI/CD use)
poetry run black --check app/ tests/
poetry run isort --check-only app/ tests/

# Run linting
poetry run flake8 app/ tests/

# Type checking
poetry run mypy app/
```

### Code Quality Standards

**Required Practices:**
- **Type Hints**: All public functions must have type hints
- **Docstrings**: All public classes and functions must have docstrings
- **Error Handling**: Comprehensive exception handling with logging
- **Security**: Input validation, SQL injection prevention, secure defaults
- **Performance**: Async/await patterns, database query optimization

**Code Example:**
```python
from typing import List, Optional
import logging
from fastapi import HTTPException
from sqlalchemy.ext.asyncio import AsyncSession

logger = logging.getLogger(__name__)

async def get_user_health_metrics(
    session: AsyncSession,
    user_id: str,
    metric_type: Optional[str] = None,
    limit: int = 100
) -> List[HealthMetric]:
    """
    Retrieve health metrics for a specific user.
    
    Args:
        session: Database session
        user_id: UUID of the user
        metric_type: Optional filter by metric type
        limit: Maximum number of records to return
        
    Returns:
        List of health metric objects
        
    Raises:
        HTTPException: If user not found or access denied
    """
    try:
        # Implementation here
        logger.info(f"Retrieved {len(metrics)} metrics for user {user_id}")
        return metrics
    except Exception as e:
        logger.error(f"Failed to retrieve metrics for user {user_id}: {str(e)}")
        raise HTTPException(status_code=500, detail="Internal server error")
```

### Database Standards

**Schema Guidelines:**
- Use UUID primary keys with `gen_random_uuid()`
- Implement proper foreign key constraints with CASCADE
- Add appropriate indexes for query patterns
- Use partitioning for time-series tables
- Include created_at/updated_at timestamps

**Migration Standards:**
- All schema changes must have Alembic migrations
- Migrations must be reversible (include downgrade)
- Test migrations on sample data
- Document breaking changes

**Query Optimization:**
- Use async SQLAlchemy patterns
- Implement proper connection pooling
- Add database indexes for query patterns
- Use BRIN indexes for time-series data
- Avoid N+1 query problems

## 🧪 Testing Requirements

### Test Coverage Standards

- **Minimum Coverage**: 85% overall code coverage
- **Critical Path Coverage**: 95% for authentication, data processing, security
- **Integration Tests**: Required for all API endpoints
- **Performance Tests**: Required for data ingestion endpoints

### Testing Framework

We use **pytest** with async support:

```bash
# Install test dependencies
poetry install --with test

# Run all tests
poetry run pytest

# Run with coverage
poetry run pytest --cov=app --cov-report=html --cov-report=term

# Run specific test categories
poetry run pytest tests/unit/          # Unit tests
poetry run pytest tests/integration/   # Integration tests
poetry run pytest tests/performance/   # Performance tests

# Run tests with specific markers
poetry run pytest -m "not slow"       # Skip slow tests
poetry run pytest -m "security"       # Security-related tests
```

### Test Categories

**Unit Tests** (`tests/unit/`):
- Test individual functions and classes
- Mock external dependencies
- Fast execution (< 1 second per test)
- Cover edge cases and error conditions

**Integration Tests** (`tests/integration/`):
- Test API endpoints end-to-end
- Use test database with real PostgreSQL
- Test authentication and authorization
- Validate complete request/response cycles

**Performance Tests** (`tests/performance/`):
- Test data ingestion with large payloads
- Database query performance
- Connection pool behavior
- Memory usage patterns

### Writing Good Tests

**Example Unit Test:**
```python
import pytest
from app.models import HealthMetric
from app.services import HealthDataProcessor

class TestHealthDataProcessor:
    @pytest.fixture
    def processor(self):
        return HealthDataProcessor()
    
    @pytest.mark.asyncio
    async def test_process_heart_rate_data(self, processor):
        """Test processing of valid heart rate data."""
        # Arrange
        raw_data = {
            "type": "heart_rate",
            "value": 72,
            "timestamp": "2024-09-08T12:00:00Z",
            "unit": "bpm"
        }
        
        # Act
        result = await processor.process_health_metric(raw_data)
        
        # Assert
        assert result.metric_type == "heart_rate"
        assert result.value == 72
        assert result.unit == "bpm"
        assert result.timestamp is not None
    
    @pytest.mark.asyncio
    async def test_process_invalid_heart_rate_raises_error(self, processor):
        """Test that invalid heart rate data raises validation error."""
        # Arrange
        invalid_data = {
            "type": "heart_rate", 
            "value": -10,  # Invalid negative heart rate
            "timestamp": "2024-09-08T12:00:00Z"
        }
        
        # Act & Assert
        with pytest.raises(ValidationError) as exc_info:
            await processor.process_health_metric(invalid_data)
        
        assert "value must be positive" in str(exc_info.value)
```

**Example Integration Test:**
```python
import pytest
from httpx import AsyncClient
from app.main import app

class TestHealthDataAPI:
    @pytest.mark.asyncio
    async def test_health_data_ingestion_success(self, async_client: AsyncClient, test_api_key):
        """Test successful health data ingestion via API."""
        # Arrange
        payload = {
            "metrics": [
                {
                    "type": "heart_rate",
                    "value": 75,
                    "timestamp": "2024-09-08T12:00:00Z",
                    "unit": "bpm"
                }
            ]
        }
        headers = {"Authorization": f"Bearer {test_api_key}"}
        
        # Act
        response = await async_client.post("/api/v1/sync", json=payload, headers=headers)
        
        # Assert
        assert response.status_code == 200
        result = response.json()
        assert result["status"] == "success"
        assert result["metrics_processed"] == 1
        assert result["errors"] == []
```

## 🔄 Pull Request Process

### Before Submitting

**Checklist:**
- [ ] Code follows style guidelines (Black, isort, flake8)
- [ ] All tests pass locally
- [ ] New functionality has comprehensive tests
- [ ] Documentation updated (if applicable)
- [ ] Database migrations created (if needed)
- [ ] Security considerations addressed
- [ ] Performance impact evaluated

### Pull Request Guidelines

**Title Format:**
```
<type>: <brief description>

Examples:
feat: add API key authentication with Redis caching
fix: resolve database connection pool exhaustion
docs: update deployment guide with Kubernetes examples
```

**Description Template:**
```markdown
## Summary
Brief description of changes and motivation.

## Changes Made
- [ ] List specific changes
- [ ] Include implementation details
- [ ] Note any breaking changes

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated  
- [ ] Manual testing completed
- [ ] Performance tested (if applicable)

## Documentation
- [ ] README updated (if applicable)
- [ ] API docs updated (if applicable)
- [ ] Architecture docs updated (if applicable)

## Security
- [ ] Security implications considered
- [ ] Input validation implemented
- [ ] Authentication/authorization verified

## Performance
- [ ] Performance impact assessed
- [ ] Database queries optimized
- [ ] Memory usage considered

## Breaking Changes
List any breaking changes and migration steps required.

## Related Issues
Closes #issue-number
Relates to #issue-number
```

### Review Process

1. **Automated Checks**: All CI/CD checks must pass
2. **Code Review**: At least one maintainer approval required
3. **Security Review**: Required for authentication, authorization, data handling
4. **Performance Review**: Required for database changes, API endpoints
5. **Documentation Review**: Required for public API changes

### After Approval

**Merging:**
- Use "Squash and merge" for feature branches
- Use "Create a merge commit" for release branches
- Delete feature branches after merge

## 📚 Documentation Standards

### Documentation Requirements

**Code Documentation:**
- All public functions and classes must have docstrings
- Use Google-style docstrings format
- Include type hints and parameter descriptions
- Document exceptions that can be raised

**API Documentation:**
- FastAPI automatically generates OpenAPI documentation
- Add detailed descriptions to endpoints, parameters, and responses
- Include example requests and responses
- Document error scenarios and status codes

**Architecture Documentation:**
- Update ARCHITECTURE.md for significant changes
- Document design decisions and trade-offs
- Include diagrams for complex systems
- Maintain up-to-date dependency information

### Documentation Examples

**Function Docstring:**
```python
async def create_api_key(
    session: AsyncSession,
    user_id: str,
    name: str,
    expires_at: Optional[datetime] = None
) -> APIKey:
    """
    Create a new API key for a user.
    
    Args:
        session: Database session for the operation
        user_id: UUID of the user who owns the API key
        name: Human-readable name for the API key
        expires_at: Optional expiration datetime, defaults to 1 year
        
    Returns:
        APIKey object with hashed key and metadata
        
    Raises:
        ValueError: If user_id is invalid
        DatabaseError: If database operation fails
        
    Example:
        >>> api_key = await create_api_key(session, user_id, "Mobile App Key")
        >>> print(api_key.key_hash)  # Argon2 hashed value
    """
```

**API Endpoint Documentation:**
```python
@router.post("/api-keys", response_model=APIKeyResponse, status_code=201)
async def create_api_key_endpoint(
    request: APIKeyCreateRequest,
    current_user: User = Depends(get_current_user),
    session: AsyncSession = Depends(get_session)
) -> APIKeyResponse:
    """
    Create a new API key for the authenticated user.
    
    Creates a new API key with Argon2 hashing and stores it securely.
    The plain text key is only returned once and cannot be recovered.
    
    **Request Body:**
    - name: Human-readable name for the API key (required)
    - expires_at: ISO 8601 datetime for expiration (optional, defaults to 1 year)
    
    **Response:**
    - api_key: The plain text API key (only returned once!)
    - key_id: UUID of the created API key
    - name: Name of the API key
    - created_at: Creation timestamp
    - expires_at: Expiration timestamp
    
    **Security:**
    - Requires authenticated user session
    - API key is Argon2 hashed before storage
    - Rate limited to 10 keys per user per hour
    """
```

## 🐛 Issue Reporting

### Bug Reports

**Use the bug report template:**
```markdown
**Bug Description**
Clear and concise description of the bug.

**To Reproduce**
Steps to reproduce the behavior:
1. Go to '...'
2. Click on '....'
3. Scroll down to '....'
4. See error

**Expected Behavior**
Clear description of what you expected to happen.

**Screenshots**
If applicable, add screenshots to help explain the problem.

**Environment:**
- OS: [e.g. Ubuntu 22.04]
- Python: [e.g. 3.13.0]
- Dependencies: [poetry show output]
- Database: [PostgreSQL version]

**Additional Context**
Add any other context about the problem here.

**Logs**
Include relevant log output (sanitize sensitive information).
```

### Feature Requests

**Use the feature request template:**
```markdown
**Feature Description**
Clear and concise description of the feature.

**Problem Statement**
What problem does this feature solve?

**Proposed Solution**
Describe the solution you'd like to see.

**Alternatives Considered**
Describe any alternative solutions you've considered.

**Implementation Notes**
Technical details, dependencies, or considerations.

**Acceptance Criteria**
- [ ] Specific criteria for feature completion
- [ ] Testing requirements
- [ ] Documentation requirements
```

## 🔒 Security Guidelines

### Security Best Practices

**Authentication & Authorization:**
- Always validate API keys using secure comparison
- Implement proper rate limiting per API key
- Log authentication failures for monitoring
- Use secure session management

**Data Protection:**
- Validate and sanitize all inputs
- Use parameterized queries to prevent SQL injection
- Implement proper CORS policies
- Hash sensitive data with appropriate algorithms (Argon2)
- Never log sensitive information

**Infrastructure Security:**
- Keep dependencies updated
- Use environment variables for secrets
- Implement proper error handling (don't expose internals)
- Use HTTPS in production
- Follow principle of least privilege

### Reporting Security Issues

**DO NOT** create public issues for security vulnerabilities.

Instead:
1. Email security issues to: [security@project.com]
2. Include detailed description and reproduction steps
3. Allow reasonable time for response and fix
4. Coordinate disclosure timeline with maintainers

## 👥 Community Guidelines

### Code of Conduct

We are committed to providing a welcoming and inclusive environment for all contributors. Please read and follow our Code of Conduct:

- **Be respectful**: Treat everyone with respect and kindness
- **Be inclusive**: Welcome newcomers and help them learn
- **Be collaborative**: Work together constructively
- **Be professional**: Keep discussions focused and productive

### Communication Channels

- **GitHub Issues**: Bug reports, feature requests, discussions
- **GitHub Discussions**: General questions, ideas, community chat
- **Pull Requests**: Code review and technical discussions

### Getting Help

**For Development Questions:**
- Check existing documentation first
- Search GitHub issues for similar problems
- Create a new issue with detailed information
- Tag maintainers for urgent issues

**For Contribution Questions:**
- Read this CONTRIBUTING guide thoroughly
- Check the project README and ARCHITECTURE docs
- Ask questions in GitHub Discussions
- Reach out to maintainers for clarification

---

## 📞 Contact

- **Project Maintainers**: [List maintainer contacts]
- **Technical Questions**: Create GitHub issues
- **Security Issues**: [security@project.com]
- **General Discussion**: GitHub Discussions

Thank you for contributing to the Health Export REST API! Your contributions help make health data management more accessible and secure for everyone.

---

**Last Updated**: September 8, 2024