/* 
  C version of Portknocker
	Usage: portknock <host> <port1> <port2> <portN>..."
	Example: portknock your.host.com 1000 2000 3000
*/

#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>
#include <netdb.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <sys/wait.h>
int main (int argc, char **argv)
{
	int j, i;
	int sd, numport = argc - 2;
	int *ports = (int *) malloc((argc-2) * sizeof(int));
	char *dst_host = argv[1];
	struct hostent* host;
	struct sockaddr_in addr;
	for(j=2;j<argc;j++) {
		ports[j-2] = atol(argv[j]);
	}
	if(argc < 3) {
		printf("Usage: %s <host> <port1> <port2> <portN>...\n",argv[0]);
		printf("\nExample: %s your.host.com 1000 2000 3000\n\n",argv[0]);
		exit(1);
	}	
	host = gethostbyname(dst_host);
	if(dst_host == NULL) {
		printf("Cannot resolve hostname\n");
		exit(1);
	}
	addr.sin_family = AF_INET;
	addr.sin_addr.s_addr = inet_addr(dst_host);
	printf("Knocking %s ...\n",dst_host);
	for(i=0;i<numport;i++) {
		addr.sin_port = htons(ports[i]);
		if((sd = socket(AF_INET, SOCK_STREAM, 0)) == -1) {
			perror ("socket");
			exit (1);
		}
		if(connect(sd,(struct sockaddr*) &addr, sizeof(addr))) {
			printf("Knocking on: %d\n",ports[i]);
		}
		close(sd);
	}
	printf("\nKnock ends!\n");
	return(0);
}
