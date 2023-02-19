#/bin/bash
java -agentlib:jdwp=transport=dt_socket,server=y,suspend=n,address=*:8200 -jar ./service/target/contactdiscovery-1.65.jar server ../CDS_config.yml

