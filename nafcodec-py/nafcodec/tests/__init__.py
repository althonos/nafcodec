from . import test_decoder, test_encoder


def load_tests(loader, suite, pattern):
    suite.addTests(loader.loadTestsFromModule(test_decoder))
    suite.addTests(loader.loadTestsFromModule(test_encoder))
    return suite
