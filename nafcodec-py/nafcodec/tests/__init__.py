from . import test_decoder, test_encoder, test_open


def load_tests(loader, suite, pattern):
    suite.addTests(loader.loadTestsFromModule(test_decoder))
    suite.addTests(loader.loadTestsFromModule(test_encoder))
    suite.addTests(loader.loadTestsFromModule(test_open))
    return suite
