/**
 * Demo Data Generator for Twins Graph
 *
 * Generates realistic example data for testing and demonstrating
 * the Twin DNA Graph visualization.
 */

(function() {
  /**
   * Generate demo twins data for testing
   */
  window.generateDemoTwinsData = function(complexity = 'medium') {
    const templates = {
      simple: {
        files: 8,
        symbols: 5,
        maxFilesPerSymbol: 3,
      },
      medium: {
        files: 20,
        symbols: 12,
        maxFilesPerSymbol: 5,
      },
      complex: {
        files: 50,
        symbols: 30,
        maxFilesPerSymbol: 8,
      },
    };

    const config = templates[complexity] || templates.medium;

    // Common symbol names that are likely to be duplicated
    const commonSymbols = [
      'Error', 'Config', 'parse', 'analyze', 'format', 'validate',
      'Node', 'resolve', 'transform', 'compile', 'render', 'process',
      'Handler', 'Builder', 'Manager', 'Service', 'Controller', 'Processor',
      'Result', 'Context', 'State', 'Options', 'Settings', 'Metadata',
      'create', 'update', 'delete', 'read', 'write', 'execute',
    ];

    // Common file path patterns
    const pathPatterns = [
      'src/core/', 'src/parser/', 'src/analyzer/', 'src/formatter/',
      'src/resolver/', 'src/utils/', 'src/lib/', 'src/types/',
      'src/handlers/', 'src/services/', 'src/controllers/', 'src/models/',
      'src/validators/', 'src/transformers/', 'src/compilers/',
    ];

    const fileExtensions = ['.rs', '.ts', '.js', '.py'];

    // Generate file paths
    const files = [];
    for (let i = 0; i < config.files; i++) {
      const pattern = pathPatterns[i % pathPatterns.length];
      const ext = fileExtensions[Math.floor(Math.random() * fileExtensions.length)];
      const fileName = `${pattern}${commonSymbols[i % commonSymbols.length].toLowerCase()}${ext}`;
      files.push(fileName);
    }

    // Generate exact twins (symbols duplicated across files)
    const exactTwins = [];
    const deadParrots = [];
    const usedSymbols = new Set();

    // Select symbols to duplicate
    const symbolsToUse = commonSymbols.slice(0, config.symbols);

    symbolsToUse.forEach(symbol => {
      // Randomly select 2-N files for this symbol
      const numFiles = 2 + Math.floor(Math.random() * (config.maxFilesPerSymbol - 1));
      const selectedFiles = [];

      // Pick random files
      for (let i = 0; i < numFiles && selectedFiles.length < files.length; i++) {
        let fileIndex;
        do {
          fileIndex = Math.floor(Math.random() * files.length);
        } while (selectedFiles.includes(files[fileIndex]));
        selectedFiles.push(files[fileIndex]);
      }

      if (selectedFiles.length >= 2) {
        exactTwins.push({
          symbol: symbol,
          files: selectedFiles,
        });

        // Generate dead parrots for each occurrence
        selectedFiles.forEach(file => {
          const line = 10 + Math.floor(Math.random() * 500);
          deadParrots.push({
            name: symbol,
            file: file,
            line: line,
          });
        });

        usedSymbols.add(symbol);
      }
    });

    return {
      exactTwins,
      deadParrots,
    };
  };

  /**
   * Generate a realistic Rust project twins data
   */
  window.generateRustProjectTwinsData = function() {
    return {
      exactTwins: [
        {
          symbol: 'Error',
          files: [
            'src/error.rs',
            'src/parser/error.rs',
            'src/analyzer/error.rs',
            'src/resolver/error.rs',
            'src/formatter/error.rs',
          ]
        },
        {
          symbol: 'Config',
          files: [
            'src/config.rs',
            'src/analyzer/config.rs',
            'src/formatter/config.rs',
          ]
        },
        {
          symbol: 'parse',
          files: [
            'src/parser/mod.rs',
            'src/analyzer/parser.rs',
            'src/utils/parse.rs',
          ]
        },
        {
          symbol: 'analyze',
          files: [
            'src/analyzer/mod.rs',
            'src/analyzer/rust.rs',
            'src/analyzer/typescript.rs',
          ]
        },
        {
          symbol: 'format',
          files: [
            'src/formatter/mod.rs',
            'src/utils/format.rs',
            'src/output/formatter.rs',
          ]
        },
        {
          symbol: 'validate',
          files: [
            'src/validator/mod.rs',
            'src/utils/validate.rs',
            'src/parser/validator.rs',
            'src/analyzer/validator.rs',
          ]
        },
        {
          symbol: 'Node',
          files: [
            'src/ast/node.rs',
            'src/parser/node.rs',
            'src/graph/node.rs',
          ]
        },
        {
          symbol: 'resolve',
          files: [
            'src/resolver/mod.rs',
            'src/analyzer/resolver.rs',
          ]
        },
        {
          symbol: 'Result',
          files: [
            'src/types/result.rs',
            'src/parser/result.rs',
            'src/analyzer/result.rs',
          ]
        },
        {
          symbol: 'Context',
          files: [
            'src/context.rs',
            'src/parser/context.rs',
            'src/analyzer/context.rs',
            'src/resolver/context.rs',
          ]
        },
      ],
      deadParrots: [
        { name: 'Error', file: 'src/error.rs', line: 12 },
        { name: 'Error', file: 'src/parser/error.rs', line: 34 },
        { name: 'Error', file: 'src/analyzer/error.rs', line: 56 },
        { name: 'Error', file: 'src/resolver/error.rs', line: 78 },
        { name: 'Error', file: 'src/formatter/error.rs', line: 90 },
        { name: 'Config', file: 'src/config.rs', line: 23 },
        { name: 'Config', file: 'src/analyzer/config.rs', line: 45 },
        { name: 'Config', file: 'src/formatter/config.rs', line: 67 },
        { name: 'parse', file: 'src/parser/mod.rs', line: 89 },
        { name: 'parse', file: 'src/analyzer/parser.rs', line: 123 },
        { name: 'parse', file: 'src/utils/parse.rs', line: 145 },
        { name: 'analyze', file: 'src/analyzer/mod.rs', line: 234 },
        { name: 'analyze', file: 'src/analyzer/rust.rs', line: 167 },
        { name: 'analyze', file: 'src/analyzer/typescript.rs', line: 289 },
        { name: 'format', file: 'src/formatter/mod.rs', line: 301 },
        { name: 'format', file: 'src/utils/format.rs', line: 323 },
        { name: 'format', file: 'src/output/formatter.rs', line: 345 },
        { name: 'validate', file: 'src/validator/mod.rs', line: 456 },
        { name: 'validate', file: 'src/utils/validate.rs', line: 478 },
        { name: 'validate', file: 'src/parser/validator.rs', line: 490 },
        { name: 'validate', file: 'src/analyzer/validator.rs', line: 512 },
        { name: 'Node', file: 'src/ast/node.rs', line: 45 },
        { name: 'Node', file: 'src/parser/node.rs', line: 67 },
        { name: 'Node', file: 'src/graph/node.rs', line: 89 },
        { name: 'resolve', file: 'src/resolver/mod.rs', line: 123 },
        { name: 'resolve', file: 'src/analyzer/resolver.rs', line: 145 },
        { name: 'Result', file: 'src/types/result.rs', line: 34 },
        { name: 'Result', file: 'src/parser/result.rs', line: 56 },
        { name: 'Result', file: 'src/analyzer/result.rs', line: 78 },
        { name: 'Context', file: 'src/context.rs', line: 90 },
        { name: 'Context', file: 'src/parser/context.rs', line: 112 },
        { name: 'Context', file: 'src/analyzer/context.rs', line: 134 },
        { name: 'Context', file: 'src/resolver/context.rs', line: 156 },
      ]
    };
  };

  /**
   * Generate a realistic TypeScript/JavaScript project twins data
   */
  window.generateTypeScriptProjectTwinsData = function() {
    return {
      exactTwins: [
        {
          symbol: 'useAuth',
          files: [
            'src/hooks/useAuth.ts',
            'src/features/auth/useAuth.ts',
            'src/components/auth/useAuth.ts',
          ]
        },
        {
          symbol: 'Button',
          files: [
            'src/components/Button.tsx',
            'src/ui/Button.tsx',
            'src/design-system/Button.tsx',
          ]
        },
        {
          symbol: 'api',
          files: [
            'src/api/index.ts',
            'src/services/api.ts',
            'src/utils/api.ts',
          ]
        },
        {
          symbol: 'formatDate',
          files: [
            'src/utils/formatDate.ts',
            'src/helpers/formatDate.ts',
            'src/lib/formatDate.ts',
          ]
        },
        {
          symbol: 'config',
          files: [
            'src/config.ts',
            'src/config/index.ts',
            'src/lib/config.ts',
            'src/utils/config.ts',
          ]
        },
      ],
      deadParrots: [
        { name: 'useAuth', file: 'src/hooks/useAuth.ts', line: 12 },
        { name: 'useAuth', file: 'src/features/auth/useAuth.ts', line: 34 },
        { name: 'useAuth', file: 'src/components/auth/useAuth.ts', line: 56 },
        { name: 'Button', file: 'src/components/Button.tsx', line: 78 },
        { name: 'Button', file: 'src/ui/Button.tsx', line: 90 },
        { name: 'Button', file: 'src/design-system/Button.tsx', line: 112 },
        { name: 'api', file: 'src/api/index.ts', line: 23 },
        { name: 'api', file: 'src/services/api.ts', line: 45 },
        { name: 'api', file: 'src/utils/api.ts', line: 67 },
        { name: 'formatDate', file: 'src/utils/formatDate.ts', line: 34 },
        { name: 'formatDate', file: 'src/helpers/formatDate.ts', line: 56 },
        { name: 'formatDate', file: 'src/lib/formatDate.ts', line: 78 },
        { name: 'config', file: 'src/config.ts', line: 12 },
        { name: 'config', file: 'src/config/index.ts', line: 34 },
        { name: 'config', file: 'src/lib/config.ts', line: 56 },
        { name: 'config', file: 'src/utils/config.ts', line: 78 },
      ]
    };
  };

  /**
   * Generate stress test data (large graph)
   */
  window.generateStressTestTwinsData = function() {
    return generateDemoTwinsData('complex');
  };

})();
