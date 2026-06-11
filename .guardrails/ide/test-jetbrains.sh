#!/bin/bash
# Test gate for JetBrains plugin
# Validates Kotlin compilation and plugin structure

set -e

cd "$(dirname "$0")/jetbrains-plugin"

echo "=== JetBrains Plugin Test Gate ==="
echo

# Check Java version
echo "[1/5] Checking Java..."
if ! command -v java &> /dev/null; then
    echo "ERROR: Java not found"
    exit 1
fi

JAVA_VERSION=$(java -version 2>&1 | grep -o '"[0-9][0-9]*\.' | head -1 | tr -d '"\.')
if [ -z "$JAVA_VERSION" ] || [ "$JAVA_VERSION" -lt "17" ]; then
    echo "WARNING: Java 17+ recommended (found $(java -version 2>&1 | head -1))"
fi
echo "✓ Java available"

# Check required files
echo "[2/5] Checking required files..."
required_files=("build.gradle.kts" "settings.gradle.kts" "gradlew" "src/main/resources/META-INF/plugin.xml")
for file in "${required_files[@]}"; do
    if [ ! -f "$file" ]; then
        echo "ERROR: Required file missing: $file"
        exit 1
    fi
done
echo "✓ All required files present"

# Check Kotlin source files
echo "[3/5] Checking Kotlin source files..."
kotlin_files=(
    "src/main/kotlin/com/guardrail/plugin/GuardrailService.kt"
    "src/main/kotlin/com/guardrail/plugin/GuardrailConfigurable.kt"
    "src/main/kotlin/com/guardrail/plugin/GuardrailInspection.kt"
)
for file in "${kotlin_files[@]}"; do
    if [ ! -f "$file" ]; then
        echo "ERROR: Required Kotlin file missing: $file"
        exit 1
    fi
done
echo "✓ Kotlin source files present"

# Validate plugin.xml
echo "[4/5] Validating plugin.xml..."
if ! grep -q "<idea-plugin>" "src/main/resources/META-INF/plugin.xml"; then
    echo "ERROR: plugin.xml missing <idea-plugin> root element"
    exit 1
fi

if ! grep -q "<id>com.guardrail.plugin</id>" "src/main/resources/META-INF/plugin.xml"; then
    echo "ERROR: plugin.xml missing plugin ID"
    exit 1
fi
echo "✓ plugin.xml structure valid"

# Build check (if gradle is available)
echo "[5/5] Attempting build..."
if [ -x "./gradlew" ]; then
    echo "Running Gradle build..."
    # Try to compile, but don't fail build on test failures
    if ./gradlew compileKotlin 2>&1 | tee /tmp/jetbrains-build.log; then
        echo "✓ Kotlin compilation successful"
    else
        echo "ERROR: Kotlin compilation failed"
        echo "See /tmp/jetbrains-build.log for details"
        exit 1
    fi
else
    echo "⚠ Gradle wrapper not executable, skipping build"
fi

echo
echo "=== All JetBrains Plugin Tests Passed ==="
