from . import test_decoder

def load_tests(loader, suite, pattern):
    suite.addTests(loader.loadTestsFromModule(test_decoder))
    return suite
