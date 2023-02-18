#/bin/bash
#java -jar ./service/target/contactdiscovery-1.53.jar server ../CDS_config.yml
java -agentlib:jdwp=transport=dt_socket,server=y,suspend=n,address=*:8200 -jar ./service/target/contactdiscovery-1.53.jar server ../CDS_config.yml

