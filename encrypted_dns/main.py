import sys

import encrypted_dns

__VERSION__ = '1.0.0'


def start():
    if sys.argv[0] == '--init':
        encrypted_dns.Config()
    elif sys.argv[0] == '-v' or sys.argv[0] == '--version':
        print("Encrypted-DNS Version:", __VERSION__)
    else:
        print("Starting Encrypted-DNS Resolver")
        dns_config_object = encrypted_dns.Config()
        dns_controller = encrypted_dns.server.Controller(dns_config_object)
        dns_controller.start()
