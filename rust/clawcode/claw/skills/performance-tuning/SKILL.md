# Performance Tuning Guidelines

## When to Use

When opencode performance needs optimization for:
- Faster response times and lower latency
- Reduced memory usage and better resource management
- Improved large project handling
- Better concurrent operation support
- Optimal configuration for your hardware and workflow

## How It Works

opencode's performance can be tuned across multiple dimensions: memory usage, CPU efficiency, disk I/O, network latency, and configuration optimization. This skill provides comprehensive guidelines for each area.

## System-Level Optimization

### 1. Memory Management

```json
{
  "memory": {
    "limits": {
      "maxHeapSize": "2G",
      "maxOldSpaceSize": "1G",
      "maxSemiSpaceSize": "256M",
      "maxNewSpaceSize": "128M"
    },
    "garbageCollection": {
      "strategy": "balanced",  // "throughput", "lowLatency", "balanced"
      "incremental": true,
      "parallel": true,
      "concurrent": true
    },
    "cache": {
      "fileSystem": {
        "enabled": true,
        "maxSize": "500MB",
        "ttl": 3600
      },
      "parsedFiles": {
        "enabled": true,
        "maxCount": 1000,
        "maxSize": "100MB"
      },
      "network": {
        "enabled": true,
        "maxSize": "50MB"
      }
    }
  }
}
```

### 2. CPU Optimization

```json
{
  "cpu": {
    "threading": {
      "workerThreads": 4,
      "ioThreads": 2,
      "maxConcurrentOperations": 10
    },
    "scheduling": {
      "priority": "normal",  // "low", "normal", "high", "realtime"
      "affinity": "auto",    // "auto" or CPU mask
      "yieldStrategy": "cooperative"
    },
    "profiling": {
      "enabled": false,
      "sampleRate": 100,  // samples per second
      "output": "cpu-profile.json"
    }
  }
}
```

### 3. Disk I/O Optimization

```json
{
  "disk": {
    "buffering": {
      "writeBufferSize": "64KB",
      "readBufferSize": "64KB",
      "asyncIO": true,
      "directIO": false
    },
    "caching": {
      "directoryCache": true,
      "fileContentCache": true,
      "metadataCache": true,
      "maxCacheSize": "200MB"
    },
    "filesystem": {
      "watchInterval": 1000,  // ms
      "recursiveWatch": true,
      "ignorePatterns": ["node_modules", ".git", "dist", "build"]
    }
  }
}
```

## Network Optimization

### 1. API Request Optimization

```json
{
  "network": {
    "api": {
      "timeout": 30000,  // ms
      "retries": 3,
      "backoff": {
        "initial": 1000,
        "multiplier": 2,
        "max": 10000
      },
      "compression": true,
      "keepAlive": true,
      "poolSize": 10
    },
    "streaming": {
      "chunkSize": 1024,
      "bufferSize": 8192,
      "timeout": 60000
    },
    "cdn": {
      "enabled": true,
      "fallback": true,
      "prefetch": true
    }
  }
}
```

### 2. Proxy and Connection Management

```json
{
  "proxy": {
    "http": "${HTTP_PROXY}",
    "https": "${HTTPS_PROXY}",
    "noProxy": "localhost,127.0.0.1",
    "tunnel": true
  },
  "dns": {
    "cache": true,
    "ttl": 300,
    "preferIPv6": false
  },
  "tls": {
    "minVersion": "TLSv1.2",
    "ciphers": "HIGH:!aNULL:!MD5",
    "sessionCache": true,
    "sessionTimeout": 300
  }
}
```

## Configuration Optimization

### 1. Startup Performance

```json
{
  "startup": {
    "lazyLoading": {
      "enabled": true,
      "modules": ["mcp", "lsp", "plugins"],
      "delay": 1000  // ms
    },
    "preload": {
      "coreModules": true,
      "frequentFiles": true,
      "recentProjects": 3
    },
    "parallelInitialization": true,
    "progressReporting": true
  }
}
```

### 2. Plugin Performance

```json
{
  "plugins": {
    "loading": {
      "parallel": true,
      "timeout": 10000,
      "maxConcurrent": 5
    },
    "isolation": {
      "sandbox": true,
      "memoryLimit": "256MB",
      "timeout": 5000
    },
    "optimization": {
      "treeShaking": true,
      "deadCodeElimination": true,
      "minification": true
    }
  }
}
```

## Large Project Optimization

### 1. File System Scanning

```json
{
  "largeProjects": {
    "fileSystem": {
      "maxFiles": 10000,
      "maxDepth": 10,
      "ignorePatterns": [
        "**/node_modules/**",
        "**/.git/**",
        "**/dist/**",
        "**/build/**",
        "**/*.min.js",
        "**/*.bundle.js"
      ],
      "scanStrategy": "incremental",  // "full", "incremental", "cached"
      "scanInterval": 5000
    },
    "indexing": {
      "enabled": true,
      "background": true,
      "priority": "low",
      "batchSize": 100
    }
  }
}
```

### 2. Memory-Efficient Operations

```json
{
  "efficientOperations": {
    "streaming": {
      "fileReading": true,
      "fileWriting": true,
      "processing": true
    },
    "chunking": {
      "largeFiles": true,
      "threshold": 1048576,  // 1MB
      "chunkSize": 65536     // 64KB
    },
    "pagination": {
      "searchResults": 50,
      "fileList": 100,
      "chatHistory": 100
    }
  }
}
```

## Monitoring and Profiling

### 1. Performance Metrics

```json
{
  "metrics": {
    "collection": {
      "enabled": true,
      "interval": 60000,  // 1 minute
      "retention": "7d"
    },
    "track": [
      "memory.heapUsed",
      "memory.external",
      "cpu.usage",
      "disk.io",
      "network.latency",
      "response.time",
      "cache.hitRate"
    ],
    "alerts": {
      "memory": {"warning": "80%", "critical": "90%"},
      "cpu": {"warning": "70%", "critical": "90%"},
      "latency": {"warning": "1000ms", "critical": "5000ms"}
    }
  }
}
```

### 2. Profiling Tools

```bash
#!/bin/bash
# ~/.opencode/profile.sh

# Memory profiling
opencode profile-memory --output memory-profile.json

# CPU profiling
opencode profile-cpu --duration 30 --output cpu-profile.json

# I/O profiling
opencode profile-io --output io-profile.json

# Network profiling
opencode profile-network --output network-profile.json

# Generate report
opencode profile-report \
  --memory memory-profile.json \
  --cpu cpu-profile.json \
  --io io-profile.json \
  --network network-profile.json \
  --output performance-report.html
```

## Hardware-Specific Tuning

### 1. Low-End Hardware

```json
{
  "lowEndHardware": {
    "memory": {
      "maxHeapSize": "512M",
      "cacheSizes": {
        "fileSystem": "50MB",
        "parsedFiles": "10MB",
        "network": "5MB"
      }
    },
    "cpu": {
      "workerThreads": 2,
      "maxConcurrentOperations": 3
    },
    "features": {
      "syntaxHighlighting": false,
      "animations": false,
      "previewPanes": false,
      "autoComplete": "basic"
    }
  }
}
```

### 2. High-End Workstation

```json
{
  "highEndWorkstation": {
    "memory": {
      "maxHeapSize": "4G",
      "cacheSizes": {
        "fileSystem": "2G",
        "parsedFiles": "500MB",
        "network": "100MB"
      }
    },
    "cpu": {
      "workerThreads": 8,
      "maxConcurrentOperations": 20
    },
    "features": {
      "parallelProcessing": true,
      "backgroundIndexing": true,
      "predictiveLoading": true,
      "advancedCaching": true
    }
  }
}
```

## Workflow-Specific Optimization

### 1. Development Workflow

```json
{
  "development": {
    "incrementalCompilation": true,
    "hotReload": true,
    "livePreview": true,
    "autoSave": {
      "enabled": true,
      "delay": 1000
    },
    "testing": {
      "parallel": true,
      "watch": true,
      "coverage": true
    }
  }
}
```

### 2. Code Review Workflow

```json
{
  "codeReview": {
    "diffOptimization": {
      "unified": true,
      "contextLines": 3,
      "ignoreWhitespace": true
    },
    "analysis": {
      "parallel": true,
      "cacheResults": true,
      "incremental": true
    },
    "presentation": {
      "sideBySide": true,
      "syntaxHighlighting": true,
      "collapsibleSections": true
    }
  }
}
```

## Advanced Optimization Techniques

### 1. Just-In-Time Compilation

```json
{
  "jit": {
    "enabled": true,
    "threshold": 100,  // Number of executions before JIT
    "optimizationLevel": 2,  // 0-3
    "profiling": {
      "enabled": true,
      "feedback": true
    }
  }
}
```

### 2. Predictive Loading

```json
{
  "predictiveLoading": {
    "enabled": true,
    "strategies": {
      "fileAccess": {
        "patternBased": true,
        "frequencyBased": true,
        "recencyBased": true
      },
      "moduleLoading": {
        "dependencyAnalysis": true,
        "usagePatterns": true
      }
    },
    "cache": {
      "preloadedFiles": 10,
      "preloadedModules": 5
    }
  }
}
```

## Benchmarking and Testing

### 1. Performance Test Suite

```bash
#!/bin/bash
# ~/.opencode/benchmark.sh

echo "Running opencode performance benchmarks..."
echo "=========================================="

# Startup time
echo -n "Startup time: "
time opencode --version > /dev/null

# Memory usage
echo -n "Memory usage: "
opencode profile-memory --quick | grep "heapUsed"

# File loading
echo -n "File loading (100KB): "
time opencode eval "fs.readFileSync('test-100kb.txt', 'utf8')" > /dev/null

# Syntax highlighting
echo -n "Syntax highlighting: "
time opencode eval "highlight('test.js')" > /dev/null

# Code analysis
echo -n "Code analysis: "
time opencode eval "analyze('test.js')" > /dev/null

echo "Benchmark complete."
```

### 2. Regression Testing

```json
{
  "regressionTesting": {
    "enabled": true,
    "tests": [
      {
        "name": "startupTime",
        "command": "opencode --version",
        "maxTime": 2000,
        "metric": "duration"
      },
      {
        "name": "memoryUsage",
        "command": "opencode profile-memory --quick",
        "maxValue": 100,
        "metric": "heapUsedMB"
      },
      {
        "name": "fileLoad",
        "command": "opencode eval \"fs.readFileSync('test.txt', 'utf8')\"",
        "maxTime": 100,
        "metric": "duration"
      }
    ],
    "schedule": "daily",
    "alertOnRegression": true
  }
}
```

## Troubleshooting Performance Issues

### 1. Diagnostic Commands

```bash
# Check current performance stats
opencode perf-stats

# Generate performance report
opencode perf-report --output report.html

# Identify bottlenecks
opencode perf-bottlenecks

# Compare configurations
opencode perf-compare config1.json config2.json

# Reset to defaults
opencode perf-reset
```

### 2. Common Issues and Solutions

**High Memory Usage:**
- Reduce cache sizes
- Enable garbage collection tuning
- Limit concurrent operations
- Disable memory-intensive features

**Slow Startup:**
- Enable lazy loading
- Reduce preloaded modules
- Disable unnecessary plugins
- Use faster storage (SSD)

**High CPU Usage:**
- Reduce worker threads
- Disable background indexing
- Limit syntax highlighting complexity
- Use simpler algorithms

**Network Latency:**
- Enable compression
- Use connection pooling
- Implement caching
- Reduce request size

## Best Practices

### 1. Regular Maintenance

- Monitor performance metrics regularly
- Clean up cache files periodically
- Update to latest versions
- Review and optimize configuration
- Remove unused plugins and extensions

### 2. Progressive Optimization

1. **Baseline**: Establish current performance metrics
2. **Identify**: Use profiling to find bottlenecks
3. **Prioritize**: Focus on highest-impact optimizations
4. **Implement**: Apply optimizations incrementally
5. **Verify**: Test after each change
6. **Monitor**: Continuously track performance

### 3. Configuration Management

- Keep configurations in version control
- Document optimization decisions
- Create environment-specific configurations
- Use inheritance for common settings
- Validate configurations regularly

## Resources

- [opencode Performance Guide](https://opencode.ai/docs/performance)
- [Node.js Performance Best Practices](https://nodejs.org/en/docs/guides/performance-best-practices)
- [Chrome DevTools Performance](https://developer.chrome.com/docs/devtools/performance/)
- [Memory Management Guide](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Memory_Management)
- [Profiling Tools Comparison](https://github.com/thlorenz/v8-perf)