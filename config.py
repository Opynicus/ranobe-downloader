import yaml

class Config:
    config = {}

    @classmethod
    def init(cls):
        with open("config.yaml", "r") as config:
            cls.config = yaml.load(config, Loader=yaml.SafeLoader)

    @classmethod
    def ews_key(cls):
        return cls.config["esjZone"]["ews_key"]

    @classmethod
    def ews_token(cls):
        return cls.config["esjZone"]["ews_token"]

    @classmethod
    def esj_root(cls):
        return cls.config["esjZone"]["esj_root_path"]

    @classmethod
    def esj_novel_url(cls):
        return cls.config["esjZone"]["esj_novel_url"]

