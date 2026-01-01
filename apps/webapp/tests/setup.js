/**
 * Test setup file
 * Runs before all tests
 */

// Mock browser APIs
global.navigator = {
  clipboard: {
    writeText: async (text) => {
      console.log('Clipboard mock:', text);
    }
  }
};

// Mock fetch if needed
global.fetch = async (url, options) => {
  console.log('Fetch mock:', url, options);
  return {
    ok: true,
    json: async () => ({ mocked: true })
  };
};
