import json
import os


class Config:

    def __init__(self):
        self.DEFAULT_CONFIG = {
            'enable_log': False,
            'enable_cache': True,

            'listen': [
                {
                    'protocol': 'udp',
                    'address': '127.0.0.1',
                    'port': 53
                },
                {
                    'protocol': 'udp',
                    'address': '127.0.0.1',
                    'port': 5301
                }
            ],

            'upstream_weight': True,
            'upstream_timeout': 30,
            'upstream_dns': [
                {
                    'protocol': 'https',
                    'address': 'cloudflare-dns.com',
                    'ip': '1.0.0.1',
                    'port': 443,
                    'weight': 0,
                    'enable_http_proxy': False,
                    'proxy_host': 'localhost',
                    'proxy_port': 8001
                },
                {
                    'protocol': 'tls',
                    'address': 'dns.google',
                    'ip': '8.8.4.4',
                    'port': 853,
                    'weight': 100
                },
                {
                    'protocol': 'udp',
                    'address': '9.9.9.9',
                    'port': 53,
                    'weight': 0
                },
                {
                    'protocol': 'tcp',
                    'address': '8.8.4.4',
                    'port': 53,
                    'weight': 0
                }
            ],

            'bootstrap_dns_address': {
                'address': '1.1.1.1',
                'port': 53
            },

            'dns_bypass': [
                "captive.apple.com",
                "connectivitycheck.gstatic.com",
                "detectportal.firefox.com",
                "msftconnecttest.com",
                "nmcheck.gnome.org",

                "pool.ntp.org",
                "time.apple.com",
                "time.asia.apple.com",
                "time.euro.apple.com",
                "time.nist.gov",
                "time.windows.com"
            ],
            'dns_bypass_china': False,

            'client_blacklist': [
                '1.0.0.1',
                '127.100.100.100'
            ],

            'force_safe_search': False,
            'hosts': {
                'localhost': '127.0.0.1'
            }
        }

        self.config = {}
        self.home = os.path.expanduser("~")
        self.file_name = self.home.rstrip('/') + '/.config/encrypted_dns/config.json'

        self.load()

    def get_config(self, key=None):
        if key is None:
            return self.config
        elif key in self.config:
            return self.config[key]

    def edit_config(self, key, value):
        if key in self.config:
            self.config[key] = value

    def get_default_config(self):
        return self.DEFAULT_CONFIG

    def load(self):
        file_init = os.path.isfile(self.file_name)
        if not file_init:
            self.config = self.get_default_config()
            self.save()
            print('Generated Config File:', self.file_name)
        else:
            config_file = open(self.file_name)
            self.config = json.loads(config_file.read())

    def save(self):
        if not os.path.exists(self.home.rstrip('/') + '/.config/'):
            os.makedirs(self.home.rstrip('/') + '/.config/')

        if not os.path.exists(self.home.rstrip('/') + '/.config/encrypted_dns'):
            os.makedirs(self.home.rstrip('/') + '/.config/encrypted_dns')

        config_json = json.dumps(self.config, indent=4)
        config_file = open(self.file_name, "w")
        config_file.write(config_json)
