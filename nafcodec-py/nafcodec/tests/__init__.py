from . import test_decoder, test_encoder, test_open, test_doctest


def load_tests(loader, suite, pattern):
    suite.addTests(loader.loadTestsFromModule(test_decoder))
    suite.addTests(loader.loadTestsFromModule(test_encoder))
    suite.addTests(loader.loadTestsFromModule(test_open))
    test_doctest.load_tests(loader, suite, pattern)
    return suite
